use proc_macro2::TokenStream;
use quote::quote;

use crate::common::rtic_traits::get_rtic_traits_mod;
use crate::parser::{ast::IdleTask, ParsedRticApp};
use crate::RticAppBuilder;

pub mod hw_task;
mod shared_resources;
pub fn generate_rtic_app(implementation: &RticAppBuilder, app: &ParsedRticApp) -> TokenStream {
    let app_mod = &app.app_name;
    let periheral_crate = &app.args.device;
    let user_includes = &app.user_includes;
    let user_code = &app.other_code;

    let pre_init = implementation.core.pre_init(app).unwrap_or_default();

    let system_critical_section_begin = implementation.core.critical_section_begin();
    let system_critical_section_end = implementation.core.critical_section_end();

    // init
    let def_init_task = &app.init.body;
    let init_task = &app.init.ident;

    // idle
    let def_idle_task = generate_idle_def(app.idle.as_ref());
    let call_idle_task = generate_idle_call(app.idle.as_ref(), implementation.core.wfi().clone());

    // hw tasks
    let call_hw_tasks_inits = app
        .hardware_tasks
        .iter()
        .map(|task| task.init_token_steam());
    let define_and_bind_hw_tasks = app
        .hardware_tasks
        .iter()
        .map(|task| task.define_and_bind_token_stream(&app.shared));

    // shared resources
    let def_shared = app.shared.generate_shared_resources_def();
    let shared_static = &app.shared.name_uppercase();
    let resource_proxies = app
        .shared
        .generate_resource_proxies(&implementation.core.impl_lock_mutex());

    // priority masks
    let priority_masks = implementation.core.compute_priority_masks(app);

    // traits
    let rtic_traits_mod = get_rtic_traits_mod();

    quote! {
        pub mod #app_mod {
            /// Include peripheral crate that defines the vector table
            use #periheral_crate as _;

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
            #(#define_and_bind_hw_tasks)*
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

                // init hardware tasks
                unsafe {#(#call_hw_tasks_inits)*}

                // user init code
                unsafe {#shared_static.write(#init_task());}

                #system_critical_section_end

                #call_idle_task
                loop{}
            }
            /// user code
            #(#user_code)*

            /// Computed priority Masks
            #priority_masks
        }
    }
}

fn generate_idle_def(idle: Option<&IdleTask>) -> TokenStream {
    if let Some(idle) = idle {
        let idle_struct = &idle.idle_struct;
        let idle_struct_impl = &idle.struct_impl;
        quote! {
            #idle_struct
            #idle_struct_impl
        }
    } else {
        quote! {}
    }
}
fn generate_idle_call(idle: Option<&IdleTask>, wfi: Option<TokenStream>) -> TokenStream {
    if let Some(idle) = idle {
        let idle_ty = &idle.struct_name;
        let idle_instance_name = &idle.instance_name;
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
