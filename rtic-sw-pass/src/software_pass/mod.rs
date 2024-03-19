mod analyze;
mod codegen;
pub(crate) mod parse;

use crate::parse::ParsedApp;
use analyze::AppAnalysis;
use proc_macro2::TokenStream;
use rtic_core::parse_utils::RticAttr;
use rtic_core::RticPass;
use crate::software_pass::codegen::CodeGen;

/// Single Core software pass
pub struct ScSoftwarePass {
    implementation: Box<dyn ScSoftwarePassImpl>
}

impl ScSoftwarePass {
    pub fn new<T: ScSoftwarePassImpl + 'static>(implementation : T) -> Self {
        Self {
            implementation: Box::new(implementation)
        }
    }
}

impl RticPass for ScSoftwarePass {
    fn run_pass(&self, params: RticAttr, app_mod: TokenStream) -> syn::Result<TokenStream> {
        let parsed = ParsedApp::parse(&params, syn::parse2(app_mod)?)?;
        let analysis = AppAnalysis::run(&parsed)?;
        let code = CodeGen::new(&parsed, &analysis, self.implementation.as_ref()).run();
        Ok(code)
    }
}

pub trait ScSoftwarePassImpl {
    /// Fill the body of the rtic internal pend() function with hardware specific implementation.
    /// Use [eprintln()] to see the `empty_body_fn` function signature
    fn fill_pend_fn(&self, empty_body_fn: syn::ItemFn) -> syn::ItemFn;
}
