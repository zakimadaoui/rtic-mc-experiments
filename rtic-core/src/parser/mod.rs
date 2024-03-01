use std::collections::HashMap;

use proc_macro2::Span;
use quote::ToTokens;
use syn::{Ident, Item, ItemFn, ItemImpl, ItemStruct, ItemUse, spanned::Spanned, Type};

use ast::*;

use crate::common::rtic_traits::HW_TASK_TRAIT_TY;

pub mod ast;

#[derive(Debug)]
pub struct ParsedRticApp {
    pub app_name: Ident,
    pub args: AppArgs,
    pub shared: SharedResources,
    pub init: InitTask,
    pub idle: Option<IdleTask>,
    pub hardware_tasks: Vec<HardwareTask>,
    pub user_includes: Vec<ItemUse>,
    pub other_code: Vec<Item>,
}

impl ParsedRticApp {
    pub fn parse(module: syn::ItemMod, args: proc_macro2::TokenStream) -> syn::Result<Self> {
        let span = module.span();
        let args = AppArgs::parse(args)?;
        let mut shared_resources = None;
        let mut inits = Vec::with_capacity(1);
        let mut idles = Vec::new();
        let mut hw_task_structs = Vec::new();
        let mut hw_task_impls: HashMap<String, ItemImpl> = HashMap::new();
        let mut user_includes = Vec::new();
        let mut other_code = Vec::new();
        let app_mod_items = module
            .content
            .ok_or(syn::Error::new(span, "Empty app module."))?
            .1;

        for item in app_mod_items {
            match item {
                Item::Fn(function) => {
                    if let Some(attr_idx) = Self::is_init(&function) {
                        inits.push((function, attr_idx))
                    } else {
                        other_code.push(function.into())
                    }
                }
                Item::Struct(strct) => {
                    if let Some(attr_idx) = Self::is_struct_with_attr(&strct, "hw_task") {
                        hw_task_structs.push((strct, attr_idx))
                    } else if let Some(attr_idx) = Self::is_struct_with_attr(&strct, "shared") {
                        if shared_resources.is_none() {
                            let _ = shared_resources.insert((strct, attr_idx));
                        } else {
                            return Err(syn::Error::new(
                                strct.ident.span(),
                                "Found more than one struct with the #[shared] attribute was found",
                            ));
                        }
                    } else if let Some(attr_idx) = Self::is_struct_with_attr(&strct, "idle") {
                        idles.push((strct, attr_idx))
                    } else {
                        other_code.push(strct.into())
                    }
                }
                Item::Impl(impl_item) => {
                    if let Some(implementor) = Self::is_task_impl(&impl_item) {
                        let _ = hw_task_impls.insert(implementor, impl_item);
                    } else {
                        other_code.push(impl_item.into())
                    }
                }
                Item::Use(use_item) => user_includes.push(use_item),
                _ => other_code.push(item),
            }
        }

        let mut app_name = module.ident;
        app_name.set_span(Span::call_site());

        let mut shared = shared_resources
            .map(|(mut strct, attr_indx)| {
                // remove the #[shared] attribute
                strct.attrs.remove(attr_indx);
                let parsed_elements = strct
                    .fields
                    .iter()
                    .map(|f| SharedElement {
                        ident: f
                            .ident
                            .clone()
                            .expect("unnamed struct is not supported for shared resources"),
                        ty: f.ty.clone(),
                        priority: 0,
                    })
                    .collect();
                SharedResources {
                    strct,
                    resources: parsed_elements,
                }
            })
            .ok_or(syn::Error::new(
                span,
                "No struct with #[shared] attribute was found",
            ))?;

        let init = Self::parse_init(inits, span, &shared)?;
        let idle = Self::parse_idle_task(idles, &hw_task_impls)?;
        let hadware_tasks = Self::link_hw_tasks_with_impl(hw_task_structs, &hw_task_impls)?;

        // update shared resources priorities
        for task in hadware_tasks.iter() {
            let task_priority = task.args.priority;
            for resource_ident in task.args.shared_idents.iter() {
                if let Some(shared_element) = shared.get_field_mut(resource_ident) {
                    if shared_element.priority < task_priority {
                        shared_element.priority = task_priority
                    }
                } else {
                    return Err(syn::Error::new(
                        span,
                        format!(
                            "The resource `{resource_ident}` was not found in `{}`",
                            shared.strct.ident
                        ),
                    ));
                }
            }
        }

        Ok(Self {
            app_name,
            args,
            shared,
            init,
            idle,
            hardware_tasks: hadware_tasks,
            user_includes,
            other_code,
        })
    }

    fn is_init(function: &ItemFn) -> Option<usize> {
        for (i, attr) in function.attrs.iter().enumerate() {
            let path = attr.meta.path();
            // we are looking for a path that has a single segment
            if path.segments.len() == 1 && path.segments[0].ident == "init" {
                return Some(i);
            }
        }
        None
    }

    /// returns the index of the `attr_name` attribute if found in the attribute list of some struct
    fn is_struct_with_attr(strct: &ItemStruct, attr_name: &str) -> Option<usize> {
        for (i, attr) in strct.attrs.iter().enumerate() {
            let path = attr.meta.path();
            if path.segments.len() == 1 && path.segments[0].ident == attr_name {
                return Some(i);
            }
        }
        None
    }

    fn is_task_impl(impl_item: &ItemImpl) -> Option<String> {
        if let Some((_, ref path, _)) = impl_item.trait_ {
            if !path.segments.is_empty()
                && path.segments[0]
                    .ident
                    .to_string()
                    .ends_with(HW_TASK_TRAIT_TY)
            {
                if let Type::Path(struct_type) = impl_item.self_ty.as_ref() {
                    let implementor_name = struct_type.path.segments[0].ident.to_string();
                    return Some(implementor_name);
                }
            }
        }
        None
    }

    /// links the hardware tasks struct definitions with their implementation part and generates a HardwareTask struct of it.
    fn link_hw_tasks_with_impl(
        task_structs: Vec<(ItemStruct, usize)>,
        task_impls: &HashMap<String, ItemImpl>,
    ) -> syn::Result<Vec<HardwareTask>> {
        let mut tasks = Vec::with_capacity(task_structs.len());
        for (mut task_struct, attr_idx) in task_structs {
            // find the task struct impl
            let struct_impl =
                task_impls
                    .get(&task_struct.ident.to_string())
                    .ok_or(syn::Error::new(
                        task_struct.span(),
                        format!("This task must implement {HW_TASK_TRAIT_TY} trait."),
                    ))?;

            // parse the hw_task attribute args
            let attr = task_struct.attrs.remove(attr_idx);
            let syn::Meta::List(args) = attr.meta else {
                return Err(syn::Error::new(
                    attr.span(),
                    "This attribute must at least have a 'binds' argument.",
                ));
            };
            let attrs = HardwareTaskArgs::parse(args.tokens)?;

            tasks.push(HardwareTask {
                args: attrs,
                task_struct,
                struct_impl: struct_impl.clone(),
            });
        }
        Ok(tasks)
    }

    fn parse_init(
        mut inits: Vec<(ItemFn, usize)>,
        module_span: Span,
        shared_resources: &SharedResources,
    ) -> syn::Result<InitTask> {
        if inits.is_empty() {
            Err(syn::Error::new(
                module_span,
                "No function with #[init] attribute was found in this module.",
            ))
        } else if inits.len() > 1 {
            Err(syn::Error::new(
                inits[1].0.span(),
                "Found more than one function with the #[init] attribute.",
            ))
        } else {
            let (mut init_fn, init_attr_idx) = inits.pop().unwrap();

            // check return type
            let expected_ret = format!("-> {}", shared_resources.strct.ident);
            let found_ret = format!("{}", init_fn.sig.output.to_token_stream());
            if found_ret != expected_ret {
                return Err(syn::Error::new(
                    init_fn.span(),
                    format!(
                        "Expected function return type to be {expected_ret}, found {found_ret}."
                    ),
                ));
            }

            // remove the [#init]
            init_fn.attrs.remove(init_attr_idx);
            Ok(InitTask {
                ident: init_fn.sig.ident.clone(),
                body: init_fn,
            })
        }
    }

    fn parse_idle_task(
        mut idles: Vec<(ItemStruct, usize)>,
        task_impls: &HashMap<String, ItemImpl>,
    ) -> syn::Result<Option<IdleTask>> {
        // TODO: change idle to no use Hardware Task Trait

        if idles.is_empty() {
            Ok(None)
        } else if idles.len() > 1 {
            Err(syn::Error::new(
                idles[1].0.span(),
                "Found more than one task with the #[idle] attribute.",
            ))
        } else {
            let (mut idle_struct, init_attr_idx) = idles.pop().unwrap();

            // find the task struct impl
            let struct_impl =
                task_impls
                    .get(&idle_struct.ident.to_string())
                    .ok_or(syn::Error::new(
                        idle_struct.span(),
                        format!("This task must implement {HW_TASK_TRAIT_TY} trait."),
                    ))?;

            // remove the [#idle]
            let _attrs = idle_struct.attrs.remove(init_attr_idx);

            let struct_name = idle_struct.ident.clone();
            let instance_name =
                Ident::new(&struct_name.to_string().to_lowercase(), Span::call_site());
            Ok(Some(IdleTask {
                attrs: IdleTaskAttrs,
                struct_name,
                instance_name,
                idle_struct,
                struct_impl: struct_impl.clone(),
            }))
        }
    }
}
