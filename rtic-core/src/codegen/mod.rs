use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use crate::analysis::Analysis;
use crate::common::rtic_functions::{get_interrupt_free_fn, INTERRUPT_FREE_FN};
use crate::common::rtic_traits::get_rtic_traits_mod;
use crate::parser::ast::{RticTask, SharedResources};
use crate::parser::{ast::IdleTask, App};
use crate::StandardPassImpl;

pub mod hw_task;
mod shared_resources;

pub struct CodeGen<'a> {
    app: &'a App,
    analysis: &'a Analysis,
    implementation: &'a dyn StandardPassImpl,
}

impl<'a> CodeGen<'a> {
    pub fn new(
        implementation: &'a dyn StandardPassImpl,
        app: &'a App,
        analysis: &'a Analysis,
    ) -> Self {
        Self {
            app,
            analysis,
            implementation,
        }
    }

    pub fn run(&self) -> TokenStream2 {
        let app = self.app;
        let implementation = self.implementation;

        let app_mod = &app.app_name;
        let peripheral_crate = &app.args.device;
        let user_includes = &app.user_includes;
        let user_code = &app.other_code;
        let interrupt_free_fn = get_interrupt_free_fn(implementation);

        // traits
        let rtic_traits_mod = get_rtic_traits_mod();

        // sub_apps
        let sub_apps = self.generate_sub_apps();

        quote! {
            pub mod #app_mod {
                /// Include peripheral crate that defines the vector table
                use #peripheral_crate as _;

                /// ================================== user includes ====================================
                #(#user_includes)*
                /// ==================================== rtic traits ====================================
                pub use rtic_traits::*;
                #rtic_traits_mod
                /// ================================== rtic functions ===================================
                /// critical section function
                #interrupt_free_fn
                /// ==================================== User code ======================================
                #(#user_code)*

                // sub applications
                #sub_apps

            }
        }
    }

    fn generate_sub_apps(&self) -> TokenStream2 {
        let implementation = self.implementation;
        let iter = self
            .app
            .sub_apps
            .iter()
            .zip(self.analysis.sub_analysis.iter());
        let args = &self.app.args;
        let apps = iter.map(|(app, analysis)| {
            let post_init = implementation.post_init(args, app, analysis);

            // init
            let def_init_task = &app.init.body;
            let init_task = &app.init.ident;

            // idle
            let def_idle_task = app.idle.as_ref().map(|idle| {
                let idle_task = idle.generate_task_def(app.shared.as_ref());
                Some(idle_task)
            });

            let call_idle_task = generate_idle_call(app.idle.as_ref(), implementation.wfi());

            // hw tasks
            let task_init_calls = app.tasks.iter().map(RticTask::task_init_call);
            let tasks_def = app
                .tasks
                .iter()
                .map(|task| task.generate_task_def(app.shared.as_ref()));

            let hw_tasks_binds = app
                .tasks
                .iter()
                .filter_map(RticTask::generate_hw_task_to_irq_binding);

            // shared resources
            let shared = app.shared.as_ref();
            let def_shared = shared.map(|shared| shared.generate_shared_resources_def());
            let shared_resources_handle = shared.map(SharedResources::name_uppercase);
            let shared_resources_handle = shared_resources_handle.iter();
            let resource_proxies = app.shared.as_ref().map(|shared| {
                shared.generate_resource_proxies(&implementation.impl_lock_mutex(app))
            });

            // priority masks
            let priority_masks = implementation.compute_priority_masks(args, app, analysis);
            let entry_name = implementation.entry_name(app.core);

            let interrupt_free = format_ident!("{}", INTERRUPT_FREE_FN);

            let doc = format!(" CORE {}", app.core);
            quote! {
                #[doc = " ===================================="]
                #[doc = #doc]
                #[doc = " ==================================== "]
                /// Computed priority Masks
                #priority_masks
                /// init task
                #def_init_task
                /// idle task
                #def_idle_task
                /// define static mut shared resources
                #def_shared
                /// proxies for accessing the shared resources
                #resource_proxies
                /// define tasks
                #(#tasks_def)*
                /// bind hw tasks to interrupts
                #(#hw_tasks_binds)*

                #[doc = r" Entry of "]
                #[doc = #doc]

                #[no_mangle]
                pub fn #entry_name() -> ! {
                    // Disable interrupts during initialization
                    #interrupt_free(||{
                        // init tasks
                        unsafe {#(#task_init_calls)*}

                        // user init code
                        let shared_resources = #init_task();
                        #(unsafe {#shared_resources_handle.write(shared_resources);})*

                        // post initialization code
                        #post_init
                    });

                    #call_idle_task
                }

            }
        });

        quote!( #(#apps)* )
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
        quote! {
            loop {
                #wfi
            }
        }
    }
}
