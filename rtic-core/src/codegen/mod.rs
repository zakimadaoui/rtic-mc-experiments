use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use crate::analysis::AppAnalysis;
use crate::common::rtic_functions::{get_interrupt_free_fn, INTERRUPT_FREE_FN};
use crate::common::rtic_traits::get_rtic_traits_mod;
use crate::parser::{ast::IdleTask, ParsedRticApp};
use crate::RticAppBuilder;

pub mod hw_task;
mod shared_resources;

pub struct CodeGen<'a> {
    app: &'a ParsedRticApp,
    analysis: &'a AppAnalysis,
    implementation: &'a RticAppBuilder,
}

impl<'a> CodeGen<'a> {
    pub fn new(
        implementation: &'a RticAppBuilder,
        app: &'a ParsedRticApp,
        analysis: &'a AppAnalysis,
    ) -> Self {
        Self {
            app,
            analysis,
            implementation,
        }
    }

    pub fn run(&self) -> TokenStream2 {
        let app = self.app;
        let analysis = self.analysis;
        let implementation = self.implementation;

        let app_mod = &app.app_name;
        let peripheral_crate = &app.args.device;
        let user_includes = &app.user_includes;
        let user_code = &app.other_code;

        let post_init = implementation
            .core
            .post_init(app, analysis)
            .unwrap_or_default();

        // TODO let system_critical_section_begin = implementation.core.critical_section_begin();
        // let system_critical_section_end = implementation.core.critical_section_end();
        let interrupt_free_fn = get_interrupt_free_fn(implementation.core.as_ref());
        let interrupt_free = format_ident!("{}", INTERRUPT_FREE_FN);

        // init
        let def_init_task = &app.init.body;
        let init_task = &app.init.ident;

        // idle
        let def_idle_task = if let Some(ref idle) = app.idle {
            idle.generate_task_def(&app.shared)
        } else {
            quote!()
        };
        let call_idle_task =
            generate_idle_call(app.idle.as_ref(), implementation.core.wfi().clone());

        // hw tasks
        let hw_tasks_inits = app
            .hardware_tasks
            .iter()
            .map(|task| task.init_token_steam());
        let hw_tasks_def = app
            .hardware_tasks
            .iter()
            .map(|task| task.generate_task_def(&app.shared));

        let hw_tasks_binds = app
            .hardware_tasks
            .iter()
            .filter_map(|task| task.generate_hw_task_to_irq_binding());

        // shared resources
        let def_shared = app.shared.generate_shared_resources_def();
        let shared_static = &app.shared.name_uppercase();
        let resource_proxies = app
            .shared
            .generate_resource_proxies(&implementation.core.impl_lock_mutex());

        // priority masks
        let priority_masks = implementation.core.compute_priority_masks(app, analysis);

        // traits
        let rtic_traits_mod = get_rtic_traits_mod();

        quote! {
            pub mod #app_mod {
                /// Include peripheral crate that defines the vector table
                use #peripheral_crate as _;

                /// ================================== user includes ====================================
                #(#user_includes)*
                /// ==================================== init task ======================================
                #def_init_task
                /// ==================================== idle task ======================================
                #def_idle_task
                /// ======================== define static mut shared resources =========================
                #def_shared
                ///====================== proxies for accessing the shared resources ====================
                #resource_proxies
                ///======================== define and bind hw tasks to interrupts ======================
                #(#hw_tasks_def)*
                #(#hw_tasks_binds)*
                /// ==================================== rtic traits ====================================
                pub use rtic_traits::*;
                #rtic_traits_mod
                /// ================================== rtic functions ===================================
                /// critical section function
                #interrupt_free_fn
                /// ======================================= main ========================================
                #[no_mangle]
                pub fn main() -> ! {
                    // Disable interrupts during initialization
                    #interrupt_free(||{
                        // init hardware and software tasks
                        unsafe {#(#hw_tasks_inits)*}
                        // post initialization code
                        #post_init
                        // user init code
                        unsafe {#shared_static.write(#init_task());}
                    });

                    #call_idle_task
                }
                /// user code
                #(#user_code)*

                /// Computed priority Masks
                #priority_masks
            }
        }
    }
}

fn generate_idle_call(idle: Option<&IdleTask>, wfi: Option<TokenStream2>) -> TokenStream2 {
    if let Some(idle) = idle {
        let idle_ty = &idle.name();
        let idle_instance_name = &idle.name_snakecase();
        quote! {
            let mut #idle_instance_name = #idle_ty::init();
            #idle_instance_name.exec();
        }
    } else {
        let wfi = wfi.unwrap_or_default();
        quote! {
            loop {
                #wfi
            }
        }
    }
}
