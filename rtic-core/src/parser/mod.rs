use std::collections::HashMap;

use proc_macro2::Span;
use quote::ToTokens;
use syn::{spanned::Spanned, Ident, Item, ItemFn, ItemImpl, ItemStruct, ItemUse, Type};

use ast::*;

use crate::analysis;
use crate::common::rtic_traits::{HWT_TRAIT_TY, IDLE_TRAIT_TY, SWT_TRAIT_TY};

pub mod ast;

#[derive(Debug)]
pub struct ParsedRticApp {
    pub app_name: Ident,
    pub args: AppArgs,
    pub shared: SharedResources,
    pub init: InitTask,
    pub idle: Option<IdleTask>,
    pub hardware_tasks: Vec<HardwareTask>,
    pub software_tasks: Vec<SoftwareTask>,
    pub user_includes: Vec<ItemUse>,
    pub other_code: Vec<Item>,
}

impl ParsedRticApp {
    pub fn parse(module: syn::ItemMod, args: proc_macro2::TokenStream) -> syn::Result<Self> {
        let span = module.span();
        let args = AppArgs::parse(args)?;
        // shared resources are a list because the framework may allow more than one shared resources struct in multicore setups,
        // but it is not decided yet how this will be handled
        let mut shared_resources = Vec::new();
        let mut inits = Vec::with_capacity(1);
        // idle tasks are a list because the framework may allow more than one idle task in multicore setups,
        // but it is not decided yet how this will be handled
        let mut idles = Vec::new();
        let mut task_structs = Vec::new();
        let mut task_impls: HashMap<String, ItemImpl> = HashMap::new();
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
                    if let Some(attr_idx) = Self::is_struct_with_attr(&strct, "task") {
                        task_structs.push((strct, attr_idx))
                    } else if let Some(attr_idx) = Self::is_struct_with_attr(&strct, "shared") {
                        shared_resources.push((strct, attr_idx));
                    } else if let Some(attr_idx) = Self::is_struct_with_attr(&strct, "idle") {
                        idles.push((strct, attr_idx))
                    } else {
                        other_code.push(strct.into())
                    }
                }
                Item::Impl(impl_item) => {
                    if let Some(implementor) = Self::is_task_impl(&impl_item) {
                        let _ = task_impls.insert(implementor, impl_item);
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

        if shared_resources.is_empty() {
            return Err(syn::Error::new(
                span,
                "No struct with #[shared] attribute was found",
            ));
        }
        let shared_resources: Vec<_> = shared_resources
            .into_iter()
            .map(|(mut strct, attr_idx)| {
                // remove the #[shared] attribute
                let attr_idx = attr_idx.clone();
                strct.attrs.remove(attr_idx);
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
            .collect();
        let mut shared = shared_resources[0].clone(); // TODO: for now other shared resource structs are ignored

        let init = Self::parse_init(inits, span, &shared)?;
        let idle = Self::construct_idle_task(idles, &task_impls)?;
        let (hardware_tasks, software_tasks) =
            Self::construct_rtic_tasks(task_structs, &task_impls)?;

        // update shared resources priorities based on task priorities and the resources they share
        analysis::update_resource_priorities(&mut shared, &hardware_tasks, &software_tasks)?;

        Ok(Self {
            app_name,
            args,
            shared,
            init,
            idle,
            hardware_tasks,
            software_tasks,
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
            if path.segments.is_empty() {
                return None;
            }

            let is_hw_task = path.segments[0].ident.to_string().ends_with(HWT_TRAIT_TY);
            let is_sw_task = path.segments[0].ident.to_string().ends_with(SWT_TRAIT_TY);
            let is_idle = path.segments[0].ident.to_string().ends_with(IDLE_TRAIT_TY);
            let is_task = is_hw_task || is_sw_task || is_idle;

            if is_task {
                if let Type::Path(struct_type) = impl_item.self_ty.as_ref() {
                    let implementor_name = struct_type.path.segments[0].ident.to_string();
                    return Some(implementor_name);
                }
            }
        }
        None
    }

    /// links the tasks struct definitions with their implementation part and generates a RticTask struct of it.
    /// The returned tasks are already split between hardware and software tasks
    fn construct_rtic_tasks(
        task_structs: Vec<(ItemStruct, usize)>,
        task_impls: &HashMap<String, ItemImpl>,
    ) -> syn::Result<(Vec<HardwareTask>, Vec<SoftwareTask>)> {
        let tasks: syn::Result<Vec<RticTask>> = task_structs
            .into_iter()
            .map(|(mut task_struct, attr_idx)| {
                // parse the task attribute args
                let attr = task_struct.attrs.remove(attr_idx);
                let syn::Meta::List(args) = attr.meta else {
                    return Err(syn::Error::new(
                        attr.span(),
                        "This attribute must at least have a 'binds' argument.",
                    ));
                };
                let args = TaskArgs::parse(args.tokens)?;

                // software or hardware task trait name that must be implemented for such task
                let task_trait_name = if args.interrupt_handler_name.is_some() {
                    HWT_TRAIT_TY
                } else {
                    SWT_TRAIT_TY
                };

                // find the task struct impl
                let struct_impl =
                    task_impls
                        .get(&task_struct.ident.to_string())
                        .ok_or(syn::Error::new(
                            task_struct.span(),
                            format!("This task must implement {task_trait_name} trait."),
                        ))?;

                Ok(RticTask {
                    args,
                    task_struct,
                    struct_impl: struct_impl.clone(),
                })
            })
            .collect();

        let (hw_tasks, sw_tasks) = tasks?
            .into_iter()
            .partition(|task| task.args.interrupt_handler_name.is_some());
        Ok((hw_tasks, sw_tasks))
    }

    fn construct_idle_task(
        mut idles: Vec<(ItemStruct, usize)>,
        task_impls: &HashMap<String, ItemImpl>,
    ) -> syn::Result<Option<IdleTask>> {
        if idles.is_empty() {
            Ok(None)
        } else {
            let (mut idle_struct, init_attr_idx) = idles.pop().unwrap(); // TODO: for now any additional idle tasks is ignored

            // find the task struct impl
            let struct_impl =
                task_impls
                    .get(&idle_struct.ident.to_string())
                    .ok_or(syn::Error::new(
                        idle_struct.span(),
                        format!("This task must implement {IDLE_TRAIT_TY} trait."),
                    ))?;

            // remove the [#idle]
            let attrs = idle_struct.attrs.remove(init_attr_idx);
            let args = if let syn::Meta::List(args) = attrs.meta {
                TaskArgs::parse(args.tokens)?
            } else {
                TaskArgs::default()
            };

            Ok(Some(IdleTask {
                args,
                task_struct: idle_struct,
                struct_impl: struct_impl.clone(),
            }))
        }
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
}
