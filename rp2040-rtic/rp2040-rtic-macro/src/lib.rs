use proc_macro::TokenStream;
use quote::quote;
use rtic_core::{AppAnalysis, CompilationPass, ParsedRticApp, RticAppBuilder, RticCoreImplementor};
use syn::{parse_quote, ItemFn};

extern crate proc_macro;

struct Rp2040Rtic;

use rtic_sw_pass::{ScSoftwarePass, ScSoftwarePassImpl};

#[proc_macro_attribute]
pub fn app(args: TokenStream, input: TokenStream) -> TokenStream {
    // use the standard software pass provided by rtic-sw-pass crate
    let sw_pass = Box::new(ScSoftwarePass::new(SwPassBackend));

    let mut builder = RticAppBuilder::new(Rp2040Rtic);
    builder.add_compilation_pass(CompilationPass::SwPass(sw_pass));
    builder.parse(args, input)
}

impl RticCoreImplementor for Rp2040Rtic {
    fn post_init(
        &self,
        app_info: &rtic_core::ParsedRticApp,
        app_analysis: &rtic_core::AppAnalysis,
    ) -> Option<proc_macro2::TokenStream> {
        let inits = app_analysis.used_irqs.iter().map(|(irq_name, priority)| {
            let peripheral_crate = &app_info.args.device;
            quote! {
                unsafe {
                    //set interrupt priority
                    #peripheral_crate::CorePeripherals::steal()
                        .NVIC
                        .set_priority(#peripheral_crate::Interrupt::#irq_name, #priority as u8);
                    //unmask interrupt
                    #peripheral_crate::NVIC::unmask(#peripheral_crate::Interrupt::#irq_name);
                }
            }
        });
        Some(quote!(#(#inits)*))
    }

    fn wfi(&self) -> Option<proc_macro2::TokenStream> {
        Some(quote! {
            unsafe { core::arch::asm!("wfi" ); }
        })
    }

    fn fill_interrupt_free_fn(&self, mut empty_body_fn: ItemFn) -> ItemFn {
        // eprintln!("{}", empty_body_fn.to_token_stream().to_string()); // enable comment to see the function signature
        let fn_body = parse_quote! {
            {
                unsafe { core::arch::asm!("cpsid i"); } // critical section begin
                let r = f();
                unsafe { core::arch::asm!("cpsie i"); } // critical section end
                r
            }
        };
        empty_body_fn.block = Box::new(fn_body);
        empty_body_fn
    }

    fn compute_priority_masks(
        &self,
        app_info: &ParsedRticApp,
        _app_analysis: &AppAnalysis,
    ) -> proc_macro2::TokenStream {
        let peripheral_crate = &app_info.args.device;

        // irq names from hadware tasks
        let irq_list_as_u32 = app_info.hardware_tasks.iter().filter_map(|t| {
            let irq_name = t.args.interrupt_handler_name.as_ref()?;
            Some(quote! { #peripheral_crate::Interrupt::#irq_name as u32, })
        });

        let mut irq_prio_map = [Vec::new(), Vec::new(), Vec::new()];
        for hw_task in app_info.hardware_tasks.iter() {
            let prio = hw_task.args.priority;
            if (1..=3).contains(&prio) {
                let Some(irq_name) = hw_task.args.interrupt_handler_name.as_ref() else {
                    continue;
                };
                irq_prio_map[(prio - 1) as usize].push(quote! {
                    #peripheral_crate::Interrupt::#irq_name as u32,
                })
            }
        }

        let mut masks = Vec::with_capacity(3);
        for priority_level in 1..=3 {
            let irq_as_u32 = &irq_prio_map[priority_level - 1];
            masks.push(quote! {
                rtic::export::create_mask([
                    #(#irq_as_u32)*
                ]),
            })
        }

        quote! {
            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            const __rtic_internal_MASK_CHUNKS: usize = rtic::export::compute_mask_chunks([
                #(#irq_list_as_u32)*
            ]);

            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            const __rtic_internal_MASKS: [rtic::export::Mask<__rtic_internal_MASK_CHUNKS>; 3] = [
                #(#masks)*
            ];
        }
    }

    fn impl_lock_mutex(&self) -> proc_macro2::TokenStream {
        quote! {
            unsafe {rtic::export::lock(resource, task_priority, CEILING, &__rtic_internal_MASKS, f);}
        }
    }
}

struct SwPassBackend;
impl ScSoftwarePassImpl for SwPassBackend {
    fn fill_pend_fn(&self, mut empty_body_fn: ItemFn) -> ItemFn {
        let body = parse_quote!({
            // taken from cortex-m implementation
            unsafe {
                (*rtic::export::NVIC::PTR).ispr[usize::from(irq_nbr / 32)]
                    .write(1 << (irq_nbr % 32))
            }
        });
        empty_body_fn.block = Box::new(body);
        empty_body_fn
    }
}
