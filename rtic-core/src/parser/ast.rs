use heck::ToSnakeCase;
use proc_macro2::Span;
use quote::ToTokens;
use syn::{parse::Parser, spanned::Spanned, Ident, ItemFn, ItemImpl, ItemStruct};

#[derive(Debug)]
pub struct InitTask {
    pub ident: Ident,
    pub body: ItemFn,
}

#[derive(Debug)]
pub struct IdleTaskAttrs;

#[derive(Debug)]
pub struct IdleTask {
    pub attrs: IdleTaskAttrs,
    pub struct_name: Ident,
    pub instance_name: Ident,
    pub idle_struct: ItemStruct,
    pub struct_impl: ItemImpl,
}

const DEFAULT_PRIORITY: u16 = 0;

#[derive(Debug)]
pub struct HardwareTaskArgs {
    pub interrupt_handler_name: syn::Path,
    pub priority: u16,
    // list of identifiers for shared resources
    pub shared_idents: Vec<Ident>,
}

impl HardwareTaskArgs {
    pub fn parse(args: proc_macro2::TokenStream) -> syn::Result<Self> {
        let args_span = args.span();
        let mut binds: Option<syn::Path> = None;
        let mut priority: Option<syn::LitInt> = None;
        let mut shared: Option<syn::ExprArray> = None;

        syn::meta::parser(|meta| {
            if meta.path.is_ident("binds") {
                binds = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("priority") {
                priority = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("shared") {
                shared = Some(meta.value()?.parse()?);
            }
            Ok(())
        })
        .parse2(args)?;

        let Some(interrupt_handler_name) = binds else {
            return Err(syn::Error::new(
                args_span,
                "A hardwar task must bind to an interrupt",
            ));
        };

        let priority = priority
            .and_then(|p| p.base10_parse().ok())
            .unwrap_or(DEFAULT_PRIORITY);

        let shared_idents = if let Some(shared) = shared {
            let mut elements = Vec::with_capacity(shared.elems.len());
            for element in shared.elems {
                let element = Ident::new(&element.to_token_stream().to_string(), Span::call_site());
                elements.push(element);
            }
            elements
        } else {
            Vec::new()
        };

        Ok(Self {
            interrupt_handler_name,
            priority,
            shared_idents,
        })
    }
}

#[derive(Debug)]
pub struct HardwareTask {
    pub args: HardwareTaskArgs,
    pub task_struct: ItemStruct,
    pub struct_impl: ItemImpl,
}

impl HardwareTask {
    pub fn name(&self) -> Ident {
        Ident::new(&self.task_struct.ident.to_string(), Span::call_site())
    }

    pub fn name_uppercase(&self) -> Ident {
        let name = self
            .task_struct
            .ident
            .to_string()
            .to_snake_case()
            .to_uppercase();
        Ident::new(&name, Span::call_site())
    }

    pub fn name_snakecase(&self) -> Ident {
        let name = self.task_struct.ident.to_string().to_snake_case();
        Ident::new(&name, Span::call_site())
    }
}

#[derive(Debug)]
pub struct SharedElement {
    pub ident: Ident,
    pub ty: syn::Type,
    pub priority: u16,
}

#[derive(Debug)]
pub struct SharedResources {
    pub strct: ItemStruct,
    pub resources: Vec<SharedElement>,
}

impl SharedResources {
    pub fn get_field_mut(&mut self, field_name: &Ident) -> Option<&mut SharedElement> {
        self.resources
            .iter_mut()
            .find(|field| &field.ident == field_name)
    }

    pub fn get_field(&self, field_name: &Ident) -> Option<&SharedElement> {
        self.resources
            .iter()
            .find(|field| &field.ident == field_name)
    }
    pub fn name_uppercase(&self) -> Ident {
        let name = self.strct.ident.to_string().to_snake_case().to_uppercase();
        Ident::new(&name, Span::call_site())
    }
}

#[derive(Debug)]
pub struct AppArgs {
    // path to peripheral crate
    pub device: syn::Path,
    pub peripherals: bool,
}

impl AppArgs {
    pub fn parse(args: proc_macro2::TokenStream) -> syn::Result<Self> {
        let args_span = args.span();
        let mut device: Option<syn::Path> = None;
        let mut peripherals: Option<syn::LitBool> = None;
        syn::meta::parser(|meta| {
            if meta.path.is_ident("device") {
                device = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("peripherals") {
                peripherals = Some(meta.value()?.parse()?);
            }
            Ok(())
        })
        .parse2(args)?;

        let Some(device) = device else {
            return Err(syn::Error::new(
                args_span,
                "device = path::to:pac must be provided.",
            ));
        };

        Ok(Self {
            device,
            peripherals: peripherals.map_or(false, |f| f.value),
        })
    }
}
