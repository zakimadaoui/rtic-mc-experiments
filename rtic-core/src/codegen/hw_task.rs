use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::parser::ast::{HardwareTask, SharedResources};

impl HardwareTask {
    /// Generates task definition, Context struct, resource proxies and binds task to appropriate interrupt
    pub fn define_and_bind_token_stream(&self, shared_resources: &SharedResources) -> TokenStream2 {
        let task_ty = &self.task_struct.ident;
        let task_static_handle = &self.name_uppercase();
        let task_irq_handler = &self.args.interrupt_handler_name;
        let task_struct = &self.task_struct;
        let task_impl = &self.struct_impl;
        let task_prio_impl = self.generate_priority_func();
        let shared_mod = shared_resources.generate_shared_for_task(self);
        quote! {
            //--------------------------------------------------------------------------------------
            static mut #task_static_handle: core::mem::MaybeUninit<#task_ty> = core::mem::MaybeUninit::uninit();
            #task_struct
            #task_impl
            #task_prio_impl
            #[allow(non_snake_case)]
            #[no_mangle]
            fn #task_irq_handler() {
                unsafe {
                    #task_static_handle.assume_init_mut().exec();
                }
            }
            #shared_mod
        }
    }

    pub fn init_token_steam(&self) -> TokenStream2 {
        let task_ty = &self.name();
        let task_static_handle = &self.name_uppercase();
        quote! { #task_static_handle.write(#task_ty::init()); }
    }

    fn generate_priority_func(&self) -> TokenStream2 {
        let task_ty = self.name();
        let task_prio = self.args.priority;
        quote! {
            impl #task_ty {
                pub const fn priority() -> u16 {
                    #task_prio
                }
            }
        }
    }
}
