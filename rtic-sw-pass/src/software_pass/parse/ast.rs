use heck::ToSnakeCase;
use quote::{format_ident, ToTokens};
use rtic_core::parse_utils::RticAttr;
use syn::{Expr, Ident, ItemImpl, ItemStruct, Lit};

pub struct AppParameters {
    pub dispatchers: Vec<Ident>,
}

impl AppParameters {
    pub fn from_attr(args: &RticAttr) -> syn::Result<Self> {
        let dispatchers =
            if let Some(&Expr::Array(ref dispatchers)) = args.elements.get("dispatchers") {
                dispatchers
                    .elems
                    .iter()
                    .map(|element| format_ident!("{}", element.to_token_stream().to_string()))
                    .collect()
            } else {
                Vec::new()
            };

        Ok(Self { dispatchers })
    }
}

pub struct SoftwareTask {
    pub attr: RticAttr,
    pub task_struct: ItemStruct,
    pub task_impl: ItemImpl,
    pub params: SoftwareTaskParams,
}

impl SoftwareTask {
    pub fn name(&self) -> &Ident {
        &self.task_struct.ident
    }

    pub fn name_uppercase(&self) -> Ident {
        let name = self
            .task_struct
            .ident
            .to_string()
            .to_snake_case()
            .to_uppercase();
        format_ident!("{name}")
    }

    pub fn name_snakecase(&self) -> Ident {
        let name = self.task_struct.ident.to_string().to_snake_case();
        format_ident!("{name}")
    }


}

pub struct SoftwareTaskParams {
    pub priority: u16,
}

impl SoftwareTaskParams {
    pub fn from_attr(attr: &RticAttr) -> Option<Self> {
        if attr.name.as_ref()? != "task" {return None};
        if let &Expr::Lit(syn::ExprLit { ref lit, .. }) = attr.elements.get("priority")?
        {
            if let Lit::Int(int) = lit {
                return Some(Self {
                    priority: int.base10_parse().ok()?
                })
            }
        }
        None
    }
}
