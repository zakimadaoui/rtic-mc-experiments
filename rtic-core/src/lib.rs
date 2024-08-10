extern crate proc_macro;

use proc_macro::TokenStream;
use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering;

use proc_macro2::{Ident, TokenStream as TokenStream2};
use syn::{parse_macro_input, ItemMod};

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
pub mod parse_utils;

mod parser;

static DEFAULT_TASK_PRIORITY: AtomicU16 = AtomicU16::new(0);

pub struct RticMacroBuilder {
    core: Box<dyn CorePassBackend>,
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
    pub fn new<T: CorePassBackend + 'static>(core_impl: T) -> Self {
        Self {
            core: Box::new(core_impl),
            pre_std_passes: Vec::new(),
            post_std_passes: Vec::new(),
        }
    }

    pub fn bind_pre_core_pass<P: RticPass + 'static>(&mut self, pass: P) -> &mut Self {
        self.pre_std_passes.push(Box::new(pass));
        self
    }

    pub fn bind_post_core_pass<P: RticPass + 'static>(&mut self, pass: P) -> &mut Self {
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

        // core pass
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
