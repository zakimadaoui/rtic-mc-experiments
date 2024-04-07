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
use crate::codegen::CodeGen;
pub use crate::parser::ast::AppArgs;
pub use crate::parser::{App, SubApp};

mod analysis;
mod codegen;
mod common;
pub mod parse_utils;

mod parser;

static DEFAULT_TASK_PRIORITY: AtomicU16 = AtomicU16::new(0);

pub struct RticAppBuilder {
    core: Box<dyn StandardPassImpl>,
    multicore_pass: Option<Box<dyn RticPass>>,
    sw_pass: Option<Box<dyn RticPass>>,
    monotonics_pass: Option<Box<dyn RticPass>>,
    other_passes: Vec<Box<dyn RticPass>>,
}
pub trait RticPass {
    fn run_pass(
        &self,
        args: TokenStream2,
        app_mod: ItemMod,
    ) -> syn::Result<(TokenStream2, ItemMod)>;
}

pub enum CompilationPass {
    MultiCorePass(Box<dyn RticPass>),
    SwPass(Box<dyn RticPass>),
    MonotonicsPass(Box<dyn RticPass>),
    Other(Box<dyn RticPass>),
}

impl RticAppBuilder {
    pub fn new<T: StandardPassImpl + 'static>(core_impl: T) -> Self {
        Self {
            core: Box::new(core_impl),
            multicore_pass: None,
            monotonics_pass: None,
            sw_pass: None,
            other_passes: Vec::new(),
        }
    }

    pub fn add_compilation_pass(&mut self, pass: CompilationPass) {
        match pass {
            CompilationPass::MultiCorePass(mono) => self.multicore_pass = Some(mono),
            CompilationPass::SwPass(sw) => self.sw_pass = Some(sw),
            CompilationPass::MonotonicsPass(mono) => self.monotonics_pass = Some(mono),
            CompilationPass::Other(pass) => self.other_passes.push(pass),
        }
    }

    pub fn build_rtic_application(self, args: TokenStream, input: TokenStream) -> TokenStream {
        // init statics
        DEFAULT_TASK_PRIORITY.store(self.core.default_task_priority(), Ordering::Relaxed);

        let mut args = TokenStream2::from(args);
        let mut app_mod = parse_macro_input!(input as ItemMod);

        // Run extra passes first in the order of their insertion
        for pass in self.other_passes {
            let (out_args, out_mod) = match pass.run_pass(args, app_mod) {
                Ok(out) => out,
                Err(e) => return e.to_compile_error().into(),
            };
            app_mod = out_mod;
            args = out_args;
        }

        // software pass
        let (args, app_mod) = if let Some(ref sw_pass) = self.sw_pass {
            match sw_pass.run_pass(args, app_mod) {
                Ok(out) => out,
                Err(e) => return e.to_compile_error().into(),
            }
        } else {
            (args, app_mod)
        };

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
    /// Code to be inserted after the call to Global init() and task init() functions
    /// This can for example enable interrupts used by the user and set their priorities
    fn post_init(
        &self,
        app_args: &AppArgs,
        app_info: &SubApp,
        app_analysis: &SubAnalysis,
    ) -> Option<TokenStream2>;

    /// Fill the body of the rtic internal critical section function with hardware specific implementation.
    /// Use [eprintln()] to see the `empty_body_fn` function signature
    fn fill_interrupt_free_fn(&self, empty_body_fn: syn::ItemFn) -> syn::ItemFn;

    /// Based on the information provided by the parsed application, such as Shared Resources priorities
    /// and Tasks priorities. Return the generated code for statically stored priority masks
    fn compute_priority_masks(
        &self,
        app_args: &AppArgs,
        app_info: &SubApp,
        app_analysis: &SubAnalysis,
    ) -> TokenStream2;

    /// Provide the body mutex implementation:
    /// impl #mutex_ty for #proxy_name {
    ///     type ResourceType = #element_ty;
    ///     fn lock(&mut self, f: impl FnOnce(&mut #element_ty)) {
    ///         const CEILING: u16 = #ceiling;
    ///         let task_priority = self.priority;
    ///         let resource = unsafe {&mut #global_resources_handle.assume_init_mut().#element_name} as *mut _;
    ///
    ///         /* Your TokenStream will be inseted here */
    ///         /* Also remember that you can have access to a global pre-computed priority mask(s) implemented by [compute_priority_masks()] */
    ///     }
    /// }
    ///
    // TODO: this interface can be improved by providing a struct like this:
    /*
    LockParams {
        resource_handle : TokenStream, // &mut resource
        current_task_priority : TokenStream, // u16 value of current task priority
        ceiling_value: TokenStream, // u16 value of resource priority
        lock_closure_handle: TokenStream, // the f(resource: &ResourceType)
    }

    // Then implementation can look like this
    quote! {
        unsafe {rtic::export::lock(#resource_handle,
                                   #current_task_priority,
                                   #ceiling_value,
                                   &__rtic_internal_MASKS, // comes from  `compute_priority_masks(...)` implementation
                                   #lock_closure_handle
                                   );}
    }
    */
    fn impl_lock_mutex(&self, app_info: &SubApp) -> TokenStream2;

    /// Implementation for WFI (Wait for interrupt) instruction to be used in default idle task
    fn wfi(&self) -> Option<TokenStream2>;

    /// Entry name for specific core
    fn entry_name(&self, core: u32) -> Ident;

    fn default_task_priority(&self) -> u16;
}
