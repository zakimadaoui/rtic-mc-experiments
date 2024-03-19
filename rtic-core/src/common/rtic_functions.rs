use quote::format_ident;
use syn::{parse_quote, ItemFn};

use crate::RticCoreImplementor;

pub const INTERRUPT_FREE_FN: &str = "__rtic_interrupt_free";

pub(crate) fn get_interrupt_free_fn(core: &dyn RticCoreImplementor) -> ItemFn {
    let fn_ident = format_ident!("{INTERRUPT_FREE_FN}");
    let critical_section_fn = parse_quote! {
        #[inline]
        pub fn #fn_ident<F, R>(f: F) -> R
        where F: FnOnce() -> R,
        {
           // Block To be implemented by the Distributor
        }
    };
    core.fill_interrupt_free_fn(critical_section_fn)
    // TODO: you can validate if the user has the correct function siganture by comparing it to the initial signature
}
