use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_quote, ImplItem};

use crate::{
    codegen::utils,
    multibin::{self, multibin_cfg_core, multibin_cfg_not_core},
    parser::ast::{HardwareTask, RticTask, SharedResources},
    StandardPassImpl,
};

impl RticTask {
    /// Generates task definition, Context struct, resource proxies and binds task to appropriate interrupt
    pub fn generate_task_def(&self, shared_resources: Option<&SharedResources>) -> TokenStream2 {
        let cfg_core = multibin::multibin_cfg_core(self.args.core);
        let task_ty = &self.task_struct.ident;
        let task_static_handle = &self.name_uppercase();
        let task_struct = &self.task_struct;
        let task_impl = &self.struct_impl;
        #[cfg(feature = "multibin")]
        let task_impl = process_task_impl(task_impl, self.args.core);

        let task_prio_impl = self.generate_priority_func();
        let shared_mod = shared_resources.map(|shared| shared.generate_shared_for_task(self));
        let current_current_fn = self.generate_current_core_fn();
        quote! {
            //--------------------------------------------------------------------------------------
            #cfg_core
            static mut #task_static_handle: core::mem::MaybeUninit<#task_ty> = core::mem::MaybeUninit::uninit();
            #task_struct

            // user implemented rtic task trait
            #task_impl

            #task_prio_impl
            #shared_mod
            #current_current_fn
        }
    }

    pub fn task_init_call(&self) -> TokenStream2 {
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

    fn generate_current_core_fn(&self) -> TokenStream2 {
        let cfg_core = multibin::multibin_cfg_core(self.args.core);
        let task_name = self.name();
        let core_type = utils::core_type(self.args.core);
        quote! {
            #cfg_core
            impl #task_name {
                const fn current_core() -> #core_type {
                    unsafe {#core_type::new()}
                }
            }
        }
    }
}

/// if "multibin" feature is enabled we need to process the task impl further.
/// Only the core that runs the task should contain the task trait implementation (init() and exec()).
/// However, because all the cores have a copy of the task struct (which must implement the task trait .. chicken-egg problem),
/// we solve this problem by reducing as much code as possible for the other cores by emptying their implemented functions.  
#[allow(unused)]
fn process_task_impl(task_impl: &syn::ItemImpl, core: u32) -> TokenStream2 {
    let cfg_core =
        multibin_cfg_core(core).expect("multibin is enabled, so this fn must not return none");
    let not_cfg_core =
        multibin_cfg_not_core(core).expect("multibin is enabled, so this fn must not return none");
    let unreachable: syn::Block = parse_quote!({
        unreachable!();
    });

    let mut emptied_task_impl = task_impl.clone();
    emptied_task_impl.items.iter_mut().for_each(|i| {
        if let ImplItem::Fn(f) = i {
            f.block = unreachable.clone();
        }
    });

    quote! {
        #cfg_core
        #task_impl
        #not_cfg_core
        #emptied_task_impl
    }
}

impl HardwareTask {
    /// Generates task definition, Context struct, resource proxies and binds task to appropriate interrupt
    pub fn generate_hw_task_to_irq_binding(
        &self,
        implementation: &dyn StandardPassImpl,
    ) -> Option<TokenStream2> {
        let cfg_core = multibin::multibin_cfg_core(self.args.core);
        let task_static_handle = &self.name_uppercase();
        let task_irq_handler = &self.args.interrupt_handler_name.clone()?;

        let defaut_task_dispatch_call = quote! {
            unsafe {#task_static_handle.assume_init_mut().exec()};
        };

        let task_dispatch_call = implementation
            .custom_task_dispatch(self.args.priority, defaut_task_dispatch_call.clone())
            .unwrap_or(defaut_task_dispatch_call);

        Some(quote! {
            #cfg_core
            #[allow(non_snake_case)]
            #[no_mangle]
            fn #task_irq_handler() {
                #task_dispatch_call
            }
        })
    }
}
