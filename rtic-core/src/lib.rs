extern crate proc_macro;

use proc_macro::TokenStream;
use std::fs;

use proc_macro2::TokenStream as TokenStream2;
use project_root::get_project_root;
use syn::{parse_macro_input, ItemMod};

pub use crate::analysis::AppAnalysis;
use crate::codegen::CodeGen;
pub use crate::parser::ast::AppArgs;
pub use crate::parser::ParsedRticApp;

mod analysis;
mod codegen;
mod common;
pub mod parse_utils;
mod parser;

/** todo:
* [ ] add pre_init code generation to enable interrupts and configure thier priorities
* [ ] init context with device and core peripherals
* [ ] run() closure in irq handlers + trait func for this

* add some analysis to check if:
[ ] idle task actaully implements RticIdleTask (probably same for Sw and Hw)
[ ] idle task must not have a priority attr and must not have binds
**/

pub struct RticAppBuilder {
    core: Box<dyn RticCoreImplementor>,
    monotonics: Option<Box<dyn RticMonotonicsImplementor>>,
}
impl RticAppBuilder {
    pub fn new<T: RticCoreImplementor + 'static>(core_impl: T) -> Self {
        Self {
            core: Box::new(core_impl),
            monotonics: None,
        }
    }
    pub fn set_monotonics_impl<T: RticMonotonicsImplementor + 'static>(
        &mut self,
        implementation: T,
    ) {
        self.monotonics = Some(Box::new(implementation));
    }

    pub fn parse(self, args: TokenStream, input: TokenStream) -> TokenStream {
        let app_module = parse_macro_input!(input as ItemMod);
        let parsed_app = match ParsedRticApp::parse(app_module.clone(), args.into()) {
            Ok(parsed) => parsed,
            Err(e) => return e.to_compile_error().into(),
        };
        let analysis = match AppAnalysis::run(&parsed_app) {
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

pub trait RticCoreImplementor {
    /// Code to be inserted before call to Global init() and task init() functions
    /// This can for example enable all interrupts used by the user
    fn pre_init(
        &self,
        app_info: &ParsedRticApp,
        app_analysis: &AppAnalysis,
    ) -> Option<TokenStream2>;

    /// Code to be used to enter a critical section.
    /// This must disable interrupts fully.
    /// It should use a compiler barier/fence to make sure that code in between [critical_section_begin()] and [critical_section_end()] is not re-ordered.
    /// It also should use a barrier instruction to not allow Out of Order execution inside critical section.
    fn critical_section_begin(&self) -> TokenStream2;

    /// Code to be used for existing a critical section.
    /// This must re-enable interrupts.
    /// It should use a compiler barier/fence to make sure that code in between [critical_section_begin()] and [critical_section_end()] is not re-ordered.
    /// It also should use a barrier instruction to not allow Out of Order execution inside critical section.
    fn critical_section_end(&self) -> TokenStream2;

    /// Based on the information provided by the parsed application, such as Shared Resources priorities
    /// and Tasks priorities. Return the generated code for statically stored priority masks
    fn compute_priority_masks(
        &self,
        app_info: &ParsedRticApp,
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
    fn impl_lock_mutex(&self) -> TokenStream2;

    /// Implementation for WFI (Wait for interrupt) instruction to be used in default idle task
    fn wfi(&self) -> Option<TokenStream2>;

    /// Priority constraints
    fn get_default_task_prio(&self) -> u16;
    fn get_min_task_prio(&self) -> u16;
    fn get_max_task_prio(&self) -> u16;
}

pub trait RticMonotonicsImplementor {}
