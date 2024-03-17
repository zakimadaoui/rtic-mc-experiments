use std::collections::HashMap;

use proc_macro2::TokenStream as TokenStream2;
use quote::format_ident;
use syn::{parse::Parser, Attribute, Meta};

pub struct RticAttr {
    pub name: Option<syn::Ident>,
    pub elements: HashMap<syn::Ident, syn::Expr>,
}

impl RticAttr {
    pub fn parse_from_attr(attribute: &Attribute) -> syn::Result<Self> {
        match attribute.meta {
            Meta::Path(ref path) => {
                let name = if path.segments.len() == 1 {
                    Some(format_ident!("{}", path.segments[0].ident))
                } else {
                    None
                };

                Ok(Self {
                    name,
                    elements: HashMap::new(),
                })
            }
            Meta::List(ref list) => {
                let name = if list.path.segments.len() == 1 {
                    Some(format_ident!("{}", list.path.segments[0].ident))
                } else {
                    None
                };

                let mut parsed = Self::parse_from_tokens(&list.tokens)?;
                parsed.name = name;
                Ok(parsed)
            }
            Meta::NameValue(_) => unreachable!(),
        }
    }

    pub fn parse_from_tokens(tokens: &TokenStream2) -> syn::Result<Self> {
        let mut elements = HashMap::new();
        syn::meta::parser(|meta| {
            let ident = meta.path.get_ident().unwrap();
            let value: syn::Expr = meta.value()?.parse()?;
            elements.insert(ident.clone(), value);
            Ok(())
        })
        .parse2(tokens.clone())?;

        Ok(Self {
            name: None,
            elements,
        })
    }
}
