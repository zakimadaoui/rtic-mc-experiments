pub(crate) mod parse;
mod analyze;
mod codegen;

use crate::parse::ParsedApp;
use proc_macro2::TokenStream;
use rtic_core::parse_utils::RticAttr;
use analyze::AppAnalysis;

pub struct ScSoftwarePass {}

impl ScSoftwarePass {
    pub fn run(params: RticAttr, app_mod: TokenStream) -> syn::Result<TokenStream> {
        let parsed = ParsedApp::parse(&params, syn::parse2(app_mod)?)?;
        let analysis = AppAnalysis::run(&parsed)?;
        let code = codegen::generate(&parsed, &analysis);
        // [ ] we still need some traits for hw specific impl for pend() and cross_pend() !
        Ok(code)
    }

}
