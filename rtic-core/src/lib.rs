extern crate proc_macro;

use proc_macro::TokenStream;

use proc_macro2::TokenStream as TokenStream2;
use syn::{ItemMod, parse_macro_input};

pub use crate::parser::ParsedRticApp;

mod codegen;
mod common;
mod parser;

/** todo:
* [ ] add pre_init code generation to enable interrupts and configure thier priorities
* [ ] init context with device and core peripherals
* [ ] shared resources for idle task (we can treat idle as a hardware task with 0 priority)
* [ ] separate trait for idle task with execute -> !
* [ ] Hook for run() closure for architectures supporting basepri
**/

pub struct RticAppBuilder {
    core: Box<dyn RticCoreImplementor>,
    monotonics: Option<Box<dyn RticMonotonicsImplementor>>,
}
impl RticAppBuilder {
    pub fn new(core_impl: Box<dyn RticCoreImplementor>) -> Self {
        Self {
            core: core_impl,
            monotonics: None,
        }
    }
    pub fn set_monotonics_impl(&mut self, implementation: Box<dyn RticMonotonicsImplementor>) {
        self.monotonics = Some(implementation);
    }

    pub fn parse(self, args: TokenStream, input: TokenStream) -> TokenStream {
        let app_module = parse_macro_input!(input as ItemMod);
        let parsed_app = match ParsedRticApp::parse(app_module.clone(), args.into()) {
            Ok(parsed) => parsed,
            Err(e) => return e.to_compile_error().into(),
        };

        codegen::generate_rtic_app(&self, &parsed_app).into()
    }
}

pub trait RticCoreImplementor {
    /// Code to be inserted before call to Global init() and task init() functions
    /// This can for example enable all interrupts used by the user
    fn pre_init(&self, app_info: &ParsedRticApp) -> Option<TokenStream2>;

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
    fn compute_priority_masks(&self, app_info: &ParsedRticApp) -> TokenStream2;

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
    fn impl_lock_mutex(&self) -> TokenStream2;

    /// Implementation for WFI (wakeup from interrupt) instruction to be used in default idle task
    fn wfi(&self) -> Option<TokenStream2>;

    /// Priority constraints
    fn get_default_task_prio(&self) -> u16;
    fn get_min_task_prio(&self) -> u16;
    fn get_max_task_prio(&self) -> u16;
}

pub trait RticMonotonicsImplementor {}
