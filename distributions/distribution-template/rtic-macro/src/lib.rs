use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::format_ident;
use rtic_auto_assign::AutoAssignPass;
use rtic_core::{AppArgs, CorePassBackend, RticMacroBuilder, SubAnalysis, SubApp};
use syn::ItemFn;

extern crate proc_macro;

struct StubBackend;

use rtic_sw_pass::SoftwarePass;

#[proc_macro_attribute]
pub fn app(args: TokenStream, input: TokenStream) -> TokenStream {
    let run_app = || {
        let mut builder = RticMacroBuilder::new(args, input);
        let _artifacts = builder.run_intermediate_pass(AutoAssignPass)?; // run auto-assign first

        let sw_pass = SoftwarePass::new(SwPassBackend); // use the standard software pass provided by rtic-sw-pass crate
        let _artifacts = builder.run_intermediate_pass(sw_pass)?; // run software pass second

        builder.run_core_pass(StubBackend)
    };
    run_app().unwrap_or_else(|e| e.into_compile_error().into())
}

// =========================================== Trait implementations ===================================================
impl CorePassBackend for StubBackend {
    fn default_task_priority(&self) -> u16 {
        todo!()
    }

    fn post_init(
        &self,
        _app_args: &AppArgs,
        _sub_app: &SubApp,
        _app_analysis: &SubAnalysis,
    ) -> Option<TokenStream2> {
        todo!()
    }

    fn populate_idle_loop(&self) -> Option<TokenStream2> {
        todo!()
    }

    fn generate_interrupt_free_fn(&self, mut _empty_body_fn: ItemFn) -> ItemFn {
        // eprintln!("{}", empty_body_fn.to_token_stream().to_string()); // enable comment to see the function signature
        todo!()
    }

    fn generate_global_definitions(
        &self,
        _app_args: &AppArgs,
        _app_info: &SubApp,
        _app_analysis: &SubAnalysis,
    ) -> Option<TokenStream2> {
        todo!()
    }

    fn generate_resource_proxy_lock_impl(
        &self,
        _app_args: &AppArgs,
        _app_info: &SubApp,
        _incomplete_lock_fn: syn::ImplItemFn,
    ) -> syn::ImplItemFn {
        todo!()
    }

    fn entry_name(&self, _core: u32) -> Ident {
        format_ident!("main")
    }

    fn wrap_task_execution(
        &self,
        _task_prio: u16,
        _dispatch_task_call: TokenStream2,
    ) -> Option<TokenStream2> {
        todo!()
    }

    // enable this only if the `multibin` feature is enabled for `rtic-core` crate
    // fn multibin_shared_macro_path(&self) -> syn::Path {
    //    todo!()
    // }

    fn pre_codegen_validation(
        &self,
        _app: &rtic_core::App,
        _analysis: &rtic_core::Analysis,
    ) -> syn::Result<()> {
        Ok(())
    }
}

struct SwPassBackend;
impl rtic_sw_pass::SwPassBackend for SwPassBackend {
    fn generate_local_pend_fn(&self, mut _empty_body_fn: ItemFn) -> ItemFn {
        todo!()
    }

    fn generate_cross_pend_fn(&self, mut _empty_body_fn: ItemFn) -> Option<ItemFn> {
        todo!()
    }
}
