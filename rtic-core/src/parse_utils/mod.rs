use std::collections::HashMap;

use proc_macro2::{Punct, Spacing, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::{parse::Parser, parse_quote, Attribute, Meta};

#[derive(Debug)]
pub struct RticAttr {
    pub name: Option<syn::Ident>,
    pub elements: HashMap<String, syn::Expr>,
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
            let value: syn::Expr = meta.value()?.parse()?;
            if let Some(ident) = meta.path.get_ident() {
                elements.insert(ident.to_string(), value);
            }
            Ok(())
        })
        .parse2(tokens.clone())?;

        Ok(Self {
            name: None,
            elements,
        })
    }
}

impl ToTokens for RticAttr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let args = self.elements.iter().map(|(name, value)| {
            let name = format_ident!("{name}");
            quote!(#name = #value)
        });
        let mut args_token_stream = TokenStream2::new();
        args_token_stream.append_separated(args, Punct::new(',', Spacing::Alone));
        let attr_name = self.name.as_ref().unwrap();
        let attribue: Attribute = parse_quote!(#[#attr_name(#args_token_stream)]);
        tokens.append_all(attribue.to_token_stream())
    }
}
