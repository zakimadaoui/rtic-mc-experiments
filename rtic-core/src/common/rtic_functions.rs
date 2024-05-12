use quote::format_ident;
use syn::{parse_quote, ImplItemFn, ItemFn};

use crate::{parser::ast::SharedElement, AppArgs, StandardPassImpl, SubApp};

pub const INTERRUPT_FREE_FN: &str = "__rtic_interrupt_free";

pub(crate) fn get_interrupt_free_fn(implementor: &dyn StandardPassImpl) -> ItemFn {
    let fn_ident = format_ident!("{INTERRUPT_FREE_FN}");
    let critical_section_fn = parse_quote! {
        #[inline]
        pub fn #fn_ident<F, R>(f: F) -> R
        where F: FnOnce() -> R,
        {
           // Block To be implemented by the Distributor
        }
    };
    implementor.impl_interrupt_free_fn(critical_section_fn)
    // TODO: you can validate if the implementor has kept the correct function signature by comparing it to the initial signature
}

pub(crate) fn get_resource_proxy_lock_fn(
    implementor: &dyn StandardPassImpl,
    app_params: &AppArgs,
    app_info: &SubApp,
    resource: &SharedElement,
    static_mut_shared_resources: &syn::Ident,
) -> ImplItemFn {
    let ceiling = resource.priority;
    let resource_ident = &resource.ident;
    let lock_fn = parse_quote! {
        fn lock(&mut self, f: impl FnOnce(&mut Self::ResourceType)) {
            // `self` refers to the resource proxy struct

            const CEILING: u16 = #ceiling; // resource priority ceiling
            let task_priority = self.task_priority; // running task priority
            let resource_ptr = unsafe { // get a mut pointer to the resource
                &mut #static_mut_shared_resources.assume_init_mut().#resource_ident
            } as *mut _;
            // TODO: continue lock implementation here
            // call for example rtic::export::lock(resource_ptr, task_priority, ...., f)
        }
    };
    implementor.impl_resource_proxy_lock(app_params, app_info, lock_fn)
    // TODO: you can validate if the implementor has kept the correct function signature by comparing it to the initial signature
}
