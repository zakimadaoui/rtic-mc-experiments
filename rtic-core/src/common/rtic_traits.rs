use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

pub const HW_TASK_TRAIT_TY: &str = "RticTask";
pub const IDLE_TRAIT_TY: &str = "RticIdleTask";
pub const MUTEX_TY: &str = "RticMutex";

pub(crate) fn get_rtic_traits_mod() -> TokenStream2 {
    let hw_task_trait = hw_task_trait();
    let _idle_trait = idle_task_trait();
    let mutex_trait = mutex_trait();
    quote! {
        /// Module defining rtic traits
        mod rtic_traits {
            #hw_task_trait
            #mutex_trait
        }
    }
}

fn hw_task_trait() -> TokenStream2 {
    let hw_task = format_ident!("{HW_TASK_TRAIT_TY}");
    quote! {
        /// Trait for a hardware task
        pub trait #hw_task {
            /// Task local variables initialization routine
            fn init() -> Self;
            /// Function to be bound to a HW Interrupt
            fn exec(&mut self);
        }
    }
}

fn idle_task_trait() -> TokenStream2 {
    let idle_task = format_ident!("{IDLE_TRAIT_TY}");
    quote! {
        /// Trait for an idle task
        pub trait #idle_task {
            /// Task local variables initialization routine
            fn init() -> Self;
            /// Function to be executing when no other task is running
            fn exec(&mut self) -> !;
        }
    }
}

fn mutex_trait() -> TokenStream2 {
    let mutex = format_ident!("{MUTEX_TY}");
    quote! {
        pub trait #mutex {
            type ResourceType;
            fn lock(&mut self, f: impl FnOnce(&mut Self::ResourceType));
        }
    }
}
