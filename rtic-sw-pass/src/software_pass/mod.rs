mod analyze;
mod codegen;
pub(crate) mod parse;

use crate::parse::App;
use crate::software_pass::codegen::CodeGen;
use analyze::Analysis;
use proc_macro2::TokenStream;
use rtic_core::parse_utils::RticAttr;
use rtic_core::RticPass;
use syn::ItemMod;

pub struct SoftwarePass {
    implementation: Box<dyn SoftwarePassImpl>,
}

impl SoftwarePass {
    pub fn new<T: SoftwarePassImpl + 'static>(implementation: T) -> Self {
        Self {
            implementation: Box::new(implementation),
        }
    }
}

impl RticPass for SoftwarePass {
    fn run_pass(&self, args: TokenStream, app_mod: ItemMod) -> syn::Result<(TokenStream, ItemMod)> {
        let params = RticAttr::parse_from_tokens(&args)?;
        let parsed = App::parse(&params, app_mod)?;
        let analysis = Analysis::run(&parsed)?;
        let code = CodeGen::new(parsed, analysis, self.implementation.as_ref()).run();
        Ok((args, code))
    }
}

/// Interface for providing the hardware specific details needed by the software pass
pub trait SoftwarePassImpl {
    /// Provide the implementation/body of the core local interrupt pending function. (implementation is hardware dependent)
    /// You can use [eprintln()] to see the `empty_body_fn` function signature
    fn impl_pend_fn(&self, empty_body_fn: syn::ItemFn) -> syn::ItemFn;

    /// (Optionally) Provide the implementation/body of the cross-core interrupt pending function. (implementation is hardware dependent)
    /// You can use [eprintln()] to see the `empty_body_fn` function signature
    fn impl_cross_pend_fn(&self, empty_body_fn: syn::ItemFn) -> Option<syn::ItemFn>;
}
