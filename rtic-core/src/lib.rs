extern crate proc_macro;

use proc_macro::TokenStream;
use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering;

use proc_macro2::{Ident, TokenStream as TokenStream2};
use syn::{parse_macro_input, ItemMod};

pub use common::rtic_functions;
pub use common::rtic_traits;

use crate::analysis::Analysis;
pub use crate::analysis::SubAnalysis;
pub use crate::codegen::multibin;
use crate::codegen::CodeGen;
pub use crate::parser::ast::AppArgs;
pub use crate::parser::{App, SubApp};

mod analysis;
mod codegen;
mod common;
pub mod parse_utils;

mod parser;

static DEFAULT_TASK_PRIORITY: AtomicU16 = AtomicU16::new(0);

pub struct RticMacroBuilder {
    core: Box<dyn StandardPassImpl>,
    pre_std_passes: Vec<Box<dyn RticPass>>,
    post_std_passes: Vec<Box<dyn RticPass>>,
}
pub trait RticPass {
    fn run_pass(
        &self,
        args: TokenStream2,
        app_mod: ItemMod,
    ) -> syn::Result<(TokenStream2, ItemMod)>;
}

impl RticMacroBuilder {
    pub fn new<T: StandardPassImpl + 'static>(core_impl: T) -> Self {
        Self {
            core: Box::new(core_impl),
            pre_std_passes: Vec::new(),
            post_std_passes: Vec::new(),
        }
    }

    pub fn bind_pre_std_pass<P: RticPass + 'static>(&mut self, pass: P) -> &mut Self {
        self.pre_std_passes.push(Box::new(pass));
        self
    }

    pub fn bind_post_std_pass<P: RticPass + 'static>(&mut self, pass: P) -> &mut Self {
        self.post_std_passes.push(Box::new(pass));
        self
    }

    pub fn build_rtic_macro(self, args: TokenStream, input: TokenStream) -> TokenStream {
        // init statics
        DEFAULT_TASK_PRIORITY.store(self.core.default_task_priority(), Ordering::Relaxed);

        let mut args = TokenStream2::from(args);
        let mut app_mod = parse_macro_input!(input as ItemMod);

        // Run extra passes first in the order of their insertion
        for pass in self.pre_std_passes {
            let (out_args, out_mod) = match pass.run_pass(args, app_mod) {
                Ok(out) => out,
                Err(e) => return e.to_compile_error().into(),
            };
            app_mod = out_mod;
            args = out_args;
        }

        // standard pass
        let mut parsed_app = match App::parse(args, app_mod) {
            Ok(parsed) => parsed,
            Err(e) => return e.to_compile_error().into(),
        };

        // update resource priorioties
        for app in parsed_app.sub_apps.iter_mut() {
            if let Err(e) = analysis::update_resource_priorities(app.shared.as_mut(), &app.tasks) {
                return e.to_compile_error().into();
            }
        }

        let analysis = match Analysis::run(&parsed_app) {
            Ok(a) => a,
            Err(e) => return e.to_compile_error().into(),
        };

        let code = CodeGen::new(self.core.as_ref(), &parsed_app, &analysis).run();

        #[cfg(feature = "debug_expand")]
        if let Ok(binary_name) = std::env::var("CARGO_BIN_NAME") {
            if let Ok(out) = project_root::get_project_root() {
                let _ = std::fs::create_dir_all(out.join("examples"));
                let _ = std::fs::write(
                    out.join(format!("examples/{binary_name}_expanded.rs")),
                    code.to_string().as_bytes(),
                );
            }
        }

        code.into()
    }
}

/// Interface for providing hw/architecture specific details for implementing the standard tasks and resources pass
pub trait StandardPassImpl {
    /// Return the default task priority to be used in idle task and tasks where priority argument is not mentioned
    fn default_task_priority(&self) -> u16;

    /// Code to be inserted after the call to Global init() and task init() functions
    /// This can for example enable interrupts used by the user and set their priorities
    fn post_init(
        &self,
        app_args: &AppArgs,
        app_info: &SubApp,
        app_analysis: &SubAnalysis,
    ) -> Option<TokenStream2>;

    /// Based on the given information about the (core-local) application, such as parsed application args, parsed sub-application and analysis,
    /// (optionally) generated the statically computed params needed to pass to the lock() function.
    /// These params could be for example masks or constant values that can later be passed to the lock() function implemented in `impl_resource_proxy_lock()`
    /// Notes:
    /// - A SubApp and SubAnalysis correspond to a single-core appliation.
    /// - This function will be called several times depending on how many cores the user defines and the implementation supports
    fn compute_lock_static_args(
        &self,
        app_args: &AppArgs,
        app_info: &SubApp,
        app_analysis: &SubAnalysis,
    ) -> Option<TokenStream2>;

    /// Complete the implementation of the lock function for a resource proxy (this implementation will be called for each resource proxy).
    /// The function signature and part of the function body are already given, do not change those, use them.
    /// Use [eprintln()] to see the `incomplete_lock_fn` signature and already provided logic inside it.
    fn impl_resource_proxy_lock(
        &self,
        app_args: &AppArgs,
        app_info: &SubApp,
        incomplete_lock_fn: syn::ImplItemFn,
    ) -> syn::ImplItemFn;

    /// (Optional) Customize how a task is dispatched when its interrupt is triggered.
    /// An example for ARM MCUs with basepri register, could be that before making the call to execute the task `dispatch_task_call`
    /// the value of basepri is saved before executing, and also restored after executing the task.
    /// If this function is not implemented, the task will execute immediately when its bound interrupt is triggered.
    fn custom_task_dispatch(
        &self,
        _task_prio: u16,
        _dispatch_task_call: TokenStream2,
    ) -> Option<TokenStream2>;

    /// Entry name for specific core
    /// This function is useful when there are multiple entries (multi-core app)
    /// and only one entry needs to be named `main`, but also the name of the other
    /// entries needs to be known at the distribution crate level for other uses.
    fn entry_name(&self, core: u32) -> Ident;

    /// Implementation for WFI (Wait for interrupt) instruction to be used in default idle task
    fn wfi(&self) -> Option<TokenStream2>;

    /// Fill the body of the critical section function with hardware specific implementation.
    /// Use [eprintln()] to see the `empty_body_fn` function signature.
    fn impl_interrupt_free_fn(&self, empty_body_fn: syn::ItemFn) -> syn::ItemFn;

    /// Provide the path to the re-exported microamp::shared macro attribute
    /// Example implementation can be
    /// ```rust
    /// fn multibin_shared_macro_path() -> syn::Path {
    ///     syn::parse_quote! { rtic::exports::microamp::shared}
    /// }
    ///
    /// This will be used to generate the use statement:
    /// ```rust
    /// use rtic::exports::microamp::shared as multibin_shared
    /// ```
    #[cfg(feature = "multibin")]
    fn multibin_shared_macro_path(&self) -> syn::Path;

    /// Make checks on the resulting parsed application and computed analysis before code generation phase is run.
    fn pre_codgen_validation(&self, _app_args: &AppArgs, _app: &App, _analysis: &Analysis) -> syn::Result<()> {
        Ok(())
    }
}
