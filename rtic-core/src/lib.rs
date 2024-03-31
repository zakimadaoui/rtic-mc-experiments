extern crate proc_macro;

use proc_macro::TokenStream;
use std::fs;

use proc_macro2::{Ident, TokenStream as TokenStream2};
use project_root::get_project_root;
use syn::{ItemMod, parse_macro_input};

pub use common::rtic_functions;
pub use common::rtic_traits;

pub use crate::analysis::AppAnalysis;
use crate::codegen::CodeGen;
use crate::parse_utils::RticAttr;
pub use crate::parser::{ParsedRticApp, RticSubApp};
pub use crate::parser::ast::AppArgs;

mod analysis;
mod codegen;
mod common;
pub mod parse_utils;

mod parser;

/** todo:
* [ ] init context with device and core peripherals
**/

pub struct RticAppBuilder {
    core: Box<dyn StandardPassImpl>,
    multicore_pass: Option<Box<dyn RticPass>>,
    sw_pass: Option<Box<dyn RticPass>>,
    monotonics_pass: Option<Box<dyn RticPass>>,
    other_passes: Vec<Box<dyn RticPass>>,
}
pub trait RticPass {
    fn run_pass(&self, params: RticAttr, app_mod: TokenStream2) -> syn::Result<TokenStream2>;
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
        // software pass
        let app_module = if let Some(ref sw_pass) = self.sw_pass {
            let app_attrs = RticAttr::parse_from_tokens(&args.clone().into()).unwrap(); // TODO: cleanup and remove unwraps
            let code = sw_pass.run_pass(app_attrs, input.into()).unwrap();
            syn::parse2(code).unwrap()
        } else {
            parse_macro_input!(input as ItemMod)
        };

        // standard pass
        let mut parsed_app = match ParsedRticApp::parse(app_module.clone(), args.into()) {
            Ok(parsed) => parsed,
            Err(e) => return e.to_compile_error().into(),
        };

        // update resource priorioties
        for app in parsed_app.sub_apps.iter_mut() {
            if let Err(e) = analysis::update_resource_priorities(app.shared.as_mut(), &app.tasks) {
                return e.to_compile_error().into();
            }
        }

        let analysis = parsed_app.sub_apps.iter().map(AppAnalysis::run).collect();
        let analysis = match analysis {
            Ok(a) => a,
            Err(e) => return e.to_compile_error().into(),
        };

        let code = CodeGen::new(&self, &parsed_app, &analysis).run();

        if let Ok(out) = get_project_root() {
            let _ = fs::create_dir_all(out.join("examples"));
            let _ = fs::write(
                out.join("examples/__expanded.rs"),
                code.to_string().as_bytes(),
            );
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
        app_info: &RticSubApp,
        app_analysis: &AppAnalysis,
    ) -> Option<TokenStream2>;

    /// Fill the body of the rtic internal critical section function with hardware specific implementation.
    /// Use [eprintln()] to see the `empty_body_fn` function signature
    fn fill_interrupt_free_fn(&self, empty_body_fn: syn::ItemFn) -> syn::ItemFn;

    /// Based on the information provided by the parsed application, such as Shared Resources priorities
    /// and Tasks priorities. Return the generated code for statically stored priority masks
    fn compute_priority_masks(
        &self,
        app_args: &AppArgs,
        app_info: &RticSubApp,
        app_analysis: &AppAnalysis,
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
    fn impl_lock_mutex(&self, app_info: &RticSubApp) -> TokenStream2;

    /// Implementation for WFI (Wait for interrupt) instruction to be used in default idle task
    fn wfi(&self) -> Option<TokenStream2>;

    /// Entry name for specific core
    fn entry_name(&self, core: u32) -> Ident;
}
