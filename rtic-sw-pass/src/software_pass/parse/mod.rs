use crate::parse::ast::{AppParameters, SoftwareTask, SoftwareTaskParams};
use proc_macro2::Ident;
use quote::format_ident;
use rtic_core::parse_utils::RticAttr;
use std::collections::HashMap;
use syn::spanned::Spanned;
use syn::{Item, ItemImpl, ItemMod, ItemStruct, Type};

pub mod ast;

pub const SWT_TRAIT_TY: &str = "RticSwTask";

pub struct ParsedApp {
    pub app_params: AppParameters,
    pub sw_tasks: Vec<SoftwareTask>,
    pub rest_of_code: Vec<Item>,
}

impl ParsedApp {
    pub fn parse(params: &RticAttr, app_mod: ItemMod) -> syn::Result<Self> {
        let app_params = ast::AppParameters::from_attr(&params)?;
        let app_mod_items = app_mod.content.unwrap_or_default().1;
        let mut sw_task_structs = Vec::new();
        let mut sw_task_impls = HashMap::new();
        let mut rest_of_code = Vec::with_capacity(app_mod_items.len());

        for item in app_mod_items {
            match item {
                Item::Struct(strct) => {
                    if let Some((params, attr)) = Self::parse_sw_task_params(&strct) {
                        sw_task_structs.push((attr, strct, params))
                    } else {
                        rest_of_code.push(Item::Struct(strct))
                    }
                }
                Item::Impl(impel) => {
                    if let Some(implementor) = Self::get_sw_task_implementor(&impel) {
                        sw_task_impls.insert(implementor.clone(), impel);
                    } else {
                        rest_of_code.push(Item::Impl(impel))
                    }
                }
                _ => rest_of_code.push(item),
            }
        }

        let mut sw_tasks = Vec::with_capacity(sw_task_structs.len());
        for (attr, task_struct, params) in sw_task_structs {
            let task_impl = sw_task_impls
                .remove(&task_struct.ident)
                .ok_or(syn::Error::new(
                    task_struct.span(),
                    format_ident!(
                        "The software task {} doesn't implement {SWT_TRAIT_TY}",
                        task_struct.ident
                    ),
                ))?;
            sw_tasks.push(SoftwareTask {
                attr,
                task_struct,
                task_impl,
                params,
            })
        }

        Ok(Self {
            app_params,
            sw_tasks,
            rest_of_code,
        })
    }

    fn parse_sw_task_params(strct: &ItemStruct) -> Option<(SoftwareTaskParams, RticAttr)> {
        for attr in strct.attrs.iter() {
            let rtic_attr = RticAttr::parse_from_attr(attr).ok();
            if let Some(rtic_attr) = rtic_attr {
                if rtic_attr.elements.get("binds").is_some() || rtic_attr.name.is_none() {
                    // sw tasks have a name ("task") and do not have "binds" argument
                    return None;
                }
                return SoftwareTaskParams::from_attr(&rtic_attr).map(|params| (params, rtic_attr));
            }
        }
        None
    }

    fn get_sw_task_implementor(impl_item: &ItemImpl) -> Option<&Ident> {
        if let Some((_, ref path, _)) = impl_item.trait_ {
            if path.segments.is_empty() {
                return None;
            }

            if path.segments[0].ident.to_string().ends_with(SWT_TRAIT_TY) {
                if let Type::Path(struct_type) = impl_item.self_ty.as_ref() {
                    let implementor_name = &struct_type.path.segments[0].ident;
                    return Some(implementor_name);
                }
            }
        }
        None
    }
}
