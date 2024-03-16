use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::analysis::AppAnalysis;
use crate::common::rtic_traits::get_rtic_traits_mod;
use crate::parser::{ast::IdleTask, ParsedRticApp};
use crate::RticAppBuilder;

pub mod hw_task;
mod shared_resources;
mod sw_task;

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

        let pre_init = implementation
            .core
            .pre_init(app, analysis)
            .unwrap_or_default();

        let system_critical_section_begin = implementation.core.critical_section_begin();
        let system_critical_section_end = implementation.core.critical_section_end();

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
            .map(|task| task.generate_hw_task_to_irq_binding());

        // sw tasks
        let sw_tasks_inits = app
            .software_tasks
            .iter()
            .map(|task| task.init_token_steam());
        let sw_tasks_def = app
            .software_tasks
            .iter()
            .map(|task| task.generate_task_def(&app.shared));

        let sw_dispatchers = sw_task::generate_sw_tasks_dispatchers(analysis);
        let sw_task_spawn_impls = app
            .software_tasks
            .iter()
            .map(|task| task.generate_sw_task_spawn_api(analysis));

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
                ///======================= define and bind sw tasks to dispatchers ======================
                #(#sw_tasks_def)*
                #(#sw_task_spawn_impls)*
                #sw_dispatchers
                /// ==================================== rtic traits ====================================
                pub use rtic_traits::*;
                #rtic_traits_mod
                /// ======================================= main ========================================
                #[no_mangle]
                pub fn main() -> ! {
                    // Disable interrupts during initialization
                    #system_critical_section_begin

                    // pre initialization code
                    #pre_init

                    // init hardware and software tasks
                    unsafe {#(#hw_tasks_inits)*}
                    unsafe {#(#sw_tasks_inits)*}

                    // user init code
                    unsafe {#shared_static.write(#init_task());}

                    #system_critical_section_end

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
