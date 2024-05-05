use proc_macro2::Span;
use rtic_core::parse_utils::RticAttr;
use std::collections::HashMap;
use syn::{Expr, Ident, ItemImpl, ItemStruct, Lit, Path};

pub struct AppParameters {
    pub dispatchers: HashMap<u32, Vec<Path>>,
    pub device: Path,
    pub cores: u32,
}

impl AppParameters {
    pub fn from_attr(args: &RticAttr) -> syn::Result<Self> {
        // parse cores arg
        let cores = if let Some(Expr::Lit(syn::ExprLit {
            lit: Lit::Int(ref cores),
            ..
        })) = args.elements.get("cores")
        {
            cores.base10_parse()?
        } else {
            1_u32
        };

        // parse peripheral crate name
        let Some(Expr::Path(p)) = args.elements.get("device") else {
            return Err(syn::Error::new(
                Span::call_site(),
                "`device` option must be provided",
            ));
        };

        // dispatchers
        let mut dispatchers = HashMap::with_capacity(cores as usize);
        if let Some(Expr::Array(ref arr)) = args.elements.get("dispatchers") {
            for (i, element) in arr.elems.iter().enumerate() {
                if let Expr::Path(ref path) = element {
                    dispatchers
                        .entry(0)
                        .or_insert(Vec::new())
                        .push(path.path.clone())
                } else if let Expr::Array(ref arr) = element {
                    let core = i;
                    let a = arr
                        .elems
                        .iter()
                        .map(|element| {
                            if let Expr::Path(ref path) = element {
                                path.path.clone()
                            } else {
                                panic!("wrong syntax")
                            }
                        })
                        .collect::<Vec<Path>>();
                    dispatchers.insert(core as u32, a);
                }
            }
        }

        if !dispatchers.is_empty() && cores as usize != dispatchers.len() {
            return Err(syn::Error::new(
                Span::call_site(),
                format!(
                    "The number of cores `{cores}` does not match the number of dispatchers `{}`",
                    dispatchers.len()
                ),
            ));
        }

        Ok(Self {
            dispatchers,
            device: p.path.clone(),
            cores,
        })
    }
}

#[derive(Debug)]
pub struct SoftwareTask {
    pub params: TaskParams,
    pub task_struct: ItemStruct,
    pub task_impl: ItemImpl,
}

impl SoftwareTask {
    pub fn name(&self) -> &Ident {
        &self.task_struct.ident
    }
}

#[derive(Debug)]
pub struct TaskParams {
    pub priority: u16,
    pub core: u32,
    pub spawn_by: u32,
}

impl TaskParams {
    pub fn from_attr(attr: &RticAttr) -> Self {
        let mut priority = 0;
        if let Some(Expr::Lit(syn::ExprLit {
            lit: Lit::Int(int), ..
        })) = attr.elements.get("priority")
        {
            priority = int.base10_parse().unwrap_or_default();
        }

        let mut core = 0;
        if let Some(Expr::Lit(syn::ExprLit {
            lit: Lit::Int(int), ..
        })) = attr.elements.get("core")
        {
            core = int.base10_parse().unwrap_or_default();
        }

        let mut spawn_by = core; // spawn_by is initially set to be the same core, unless the user chooses otherwize
        if let Some(Expr::Lit(syn::ExprLit {
            lit: Lit::Int(int), ..
        })) = attr.elements.get("spawn_by")
        {
            spawn_by = int.base10_parse().unwrap_or_default();
        }

        Self {
            priority,
            core,
            spawn_by,
        }
    }
}
