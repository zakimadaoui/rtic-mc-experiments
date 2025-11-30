//! This is a re-usable crate that captures the **hardware agnostic** proc-macro logic for RTIC's tasks and resources syntax model (both single and multicore syntax). I.e the user code parsing, analysis and generation for hardware tasks, local resources, shared resources and their locking mechanism, system init and idle tasks.
//!
//! In addition, this crate provides utilities for building the actual RTIC crate (known as an **RTIC distribution**) that exports the RTIC framework attribute procedural macro for a specific target hardware architecture. Further more, the same utilizes allow extending the **core syntax** provided by this crate through the concept of **Compilation passes**. As a result, this crate is not meant to be used directly by users who want to write RTIC applications, instead it is used by **RTIC distribution** implementors.
//!
#![doc = include_str!("../../compilation_passes/compilation_passes.md")]
#![doc = include_str!("../../distributions/rtic_distributions.md")]
//!
//! ### Guidelines for implementing new distributions, links, and link to template distribution
//! TODO...

extern crate proc_macro;

use proc_macro::TokenStream;
use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering;

use proc_macro2::{Ident, TokenStream as TokenStream2};
use syn::ItemMod;

pub use common_internal::rtic_functions;
pub use common_internal::rtic_traits;

pub use analysis::{Analysis, SubAnalysis};
pub use backend::CorePassBackend;
pub use codegen::multibin;
use codegen::CodeGen;
pub use parser::ast::AppArgs;
pub use parser::{App, SubApp};

mod analysis;
mod backend;
mod codegen;
mod common_internal;
pub mod errors;
pub mod parse_utils;

mod parser;

static DEFAULT_TASK_PRIORITY: AtomicU16 = AtomicU16::new(0);

/// A trait that allows defining a **Compilation Pass**.
///
/// A **Compilation Pass** can be thought of as a (partial) proc-macro that expands parts of the user RTIC application
/// once all the compilation passes provided using [RticMacroBuilder::bind_pre_core_pass] and
/// [RticMacroBuilder::bind_post_core_pass] are run. The resulting code should be comprised only of *init*, *idle* ,
/// *shared resources* and *tasks* (that may be bound to interrupts) that share those resources. The **Core Pass**
/// then will take over from there to generate all the necessary logic and expand the user application further to an
/// application understandable by the Rust compiler.
pub trait RticPass {
    /// Associated type that defines the artifacts left after running the compilation pass.
    /// Those artifacts can, for example, be used by subsequent passes or their backends  
    type PassArtifacts;
    /// Runs the (partial) proc-macro logic that allows extending the basic RTIC syntax
    fn run_pass(
        &self,
        args: TokenStream2,
        app_mod: ItemMod,
    ) -> syn::Result<(TokenStream2, ItemMod, Self::PassArtifacts)>;

    /// Returns a human readable name/alias used to identify the pass. This identifier will show np in errors for example
    /// to help knowing exactly which compilation pass has failed in that case.
    fn pass_name(&self) -> &str;
}

/// This should be used to compose an **RTIC distribution**. In other words, it allows building the RTIC **app** macro
/// By providing the necessary low-level hardware bindings and binding additional **Compilation Passes**
/// in the case syntax extensions are desired.
pub struct RticMacroBuilder {
    args: Option<TokenStream2>,
    app_mod: Option<ItemMod>,
}

impl RticMacroBuilder {
    pub fn new(args: TokenStream, input: TokenStream) -> Self {
        let args = TokenStream2::from(args);
        let app_mod =
            syn::parse::<ItemMod>(input).expect("Should be ItemMod. TODO: return error instead..");
        Self {
            args: Some(args),
            app_mod: Some(app_mod),
        }
    }

    /// Runs a compilation pass to perform an intermediate user application expansion.  
    pub fn run_intermediate_pass<Artifacts, P: RticPass<PassArtifacts = Artifacts>>(
        &mut self,
        pass: P,
    ) -> syn::Result<Artifacts> {
        // safe to unwrap as the `args` and `app_mod` always have a value.
        let (args, app_mod) = (self.args.take().unwrap(), self.app_mod.take().unwrap());
        let (args, app_mod, artifacts) = pass.run_pass(args, app_mod).inspect_err(|_| {
            eprintln!(
                "An error occurred during the `{}` compilation pass",
                pass.pass_name()
            )
        })?;

        self.args = Some(args);
        self.app_mod = Some(app_mod);
        Ok(artifacts)
    }

    /// Performs the final user application expansion in which the application code is only comprised of init, idle, hw tasks and resources
    /// # Returns
    /// On success, the [TokenStream] representing the fully expanded user application  
    pub fn run_core_pass<B: CorePassBackend + 'static>(
        self,
        core_backend: B,
    ) -> syn::Result<TokenStream> {
        // init statics
        DEFAULT_TASK_PRIORITY.store(core_backend.default_task_priority(), Ordering::Relaxed);

        // parse user application comprised of init, idle, hw tasks and resources
        let mut parsed_app =  App::parse(self.args.unwrap(), self.app_mod.unwrap()).inspect_err(|_| eprintln!("An error occurred during the `core` compilation pass during the user code `parsing` phase."))?;

        // update resource ceilings and gather more information about the application
        let analysis = Analysis::run(&mut parsed_app).inspect_err(|_| eprintln!("An error occurred during the `core` compilation pass  during the user code `analysis` phase."))?;

        // Before starting code generation, ask distribution for further checks
        core_backend.pre_codegen_validation(&parsed_app, &analysis)?;

        let code = CodeGen::new(&core_backend, &parsed_app, &analysis).run();

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
        Ok(code.into())
    }
}
