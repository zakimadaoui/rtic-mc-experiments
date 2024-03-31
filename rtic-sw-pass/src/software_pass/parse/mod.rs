use crate::parse::ast::{AppParameters, SoftwareTask, SoftwareTaskParams};
use proc_macro2::Ident;
use rtic_core::parse_utils::RticAttr;
use std::collections::HashMap;
use syn::spanned::Spanned;
use syn::{Item, ItemImpl, ItemMod, ItemStruct, Type, Visibility};

pub mod ast;

pub const SWT_TRAIT_TY: &str = "RticSwTask";

/// Type to represent a sub application (application on a single core)
pub struct SubApp {
    pub core: u32,
    pub dispatchers: Vec<syn::Path>,
    pub sw_tasks: Vec<SoftwareTask>,
}

/// Type to represent an RTIC application (withing software pass context)
/// The application contains one or more sub-applications (one application per-core)
pub struct App {
    pub mod_visibility: Visibility,
    pub mod_ident: Ident,
    pub app_params: AppParameters,
    /// a list of sub-applications, one sub-app per core.
    pub sub_apps: Vec<SubApp>,
    pub rest_of_code: Vec<Item>,
}

impl App {
    pub fn parse(params: &RticAttr, mut app_mod: ItemMod) -> syn::Result<Self> {
        let app_params = AppParameters::from_attr(params)?;
        let app_mod_items = app_mod.content.take().unwrap_or_default().1;
        let mut sw_task_structs = Vec::new();
        let mut sw_task_impls = HashMap::new();
        let mut rest_of_code = Vec::with_capacity(app_mod_items.len());

        for item in app_mod_items {
            match item {
                Item::Struct(strct) => {
                    if let Some(attr_idx) = Self::is_struct_with_attr(&strct, "sw_task") {
                        sw_task_structs.push((strct, attr_idx))
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

        let cores = app_params.cores;
        let mut sw_tasks = HashMap::with_capacity(cores as usize);
        for (task_struct, attr_idx) in sw_task_structs {
            let task_impl = sw_task_impls
                .remove(&task_struct.ident)
                .ok_or(syn::Error::new(
                    task_struct.span(),
                    format!(
                        "The software task {} doesn't implement {SWT_TRAIT_TY}",
                        task_struct.ident
                    ),
                ))?;

            let attrs = RticAttr::parse_from_attr(&task_struct.attrs[attr_idx])?;
            let params = SoftwareTaskParams::from_attr(&attrs);
            let task = SoftwareTask {
                params,
                task_struct,
                task_impl,
            };
            sw_tasks
                .entry(task.params.core)
                .or_insert(Vec::new())
                .push(task);
        }

        let mut sub_apps = Vec::with_capacity(cores as usize);
        for core in 0..cores {
            let dispatchers = app_params.dispatchers.get(&core).cloned().unwrap_or_default();
            sub_apps.push(SubApp {
                core,
                dispatchers,
                sw_tasks: sw_tasks.remove(&core).unwrap_or_default(),
            })
        }

        Ok(Self {
            mod_ident: app_mod.ident,
            mod_visibility: app_mod.vis,
            app_params,
            sub_apps,
            rest_of_code,
        })
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
