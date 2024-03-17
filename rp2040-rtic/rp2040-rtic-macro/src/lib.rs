use proc_macro::TokenStream;
use quote::quote;
use rtic_core::{AppAnalysis, ParsedRticApp, RticAppBuilder, RticCoreImplementor};
extern crate proc_macro;

struct Rp2040Rtic;

impl RticCoreImplementor for Rp2040Rtic {
    fn get_default_task_prio(&self) -> u16 {
        0
    }

    fn get_min_task_prio(&self) -> u16 {
        1
    }

    fn get_max_task_prio(&self) -> u16 {
        3
    }

    fn pre_init(&self, _app_info: &rtic_core::ParsedRticApp, _app_analysis: &rtic_core::AppAnalysis) -> Option<proc_macro2::TokenStream> {
        // TODO: later initialize Interrupts here
        None
    }

    fn critical_section_begin(&self) -> proc_macro2::TokenStream {
        quote! {
            unsafe { core::arch::asm!("cpsid i"); }
        }
    }

    fn critical_section_end(&self) -> proc_macro2::TokenStream {
        quote! {
            unsafe { core::arch::asm!("cpsie i" ); }
        }
    }

    fn wfi(&self) -> Option<proc_macro2::TokenStream> {
        Some(quote! {
            unsafe { core::arch::asm!("wfi" ); }
        })
    }

    fn compute_priority_masks(&self, app_info: &ParsedRticApp, app_analysis: &AppAnalysis) -> proc_macro2::TokenStream {
        let peripheral_crate = &app_info.args.device;

        // irq names from hadware tasks
        let irq_list_as_u32 = app_info.hardware_tasks.iter().map(|t| {
            let irq_name = &t.args.interrupt_handler_name;
            quote! { #peripheral_crate::Interrupt::#irq_name as u32, }
        });

        // irq names from software tasks
        let irq_list_as_u32 = irq_list_as_u32.chain(app_analysis.dispatcher_priorities.values().map(|irq| {
            quote! { #peripheral_crate::Interrupt::#irq as u32, }
        }));

        let mut irq_prio_map = [Vec::new(), Vec::new(), Vec::new()];
        for hw_task in app_info.hardware_tasks.iter() {
            let prio = hw_task.args.priority;
            if (1..=3).contains(&prio) {
                let irq_name = hw_task.args.interrupt_handler_name.as_ref().unwrap(); //safe to unwarap hw task irq ident
                irq_prio_map[(prio - 1) as usize].push(quote! {
                    #peripheral_crate::Interrupt::#irq_name as u32,
                })
            }
        }

        for (prio, dispatcher) in app_analysis.dispatcher_priorities.iter() {
            if (1_u16..=3).contains(&prio) {
                irq_prio_map[(prio - 1) as usize].push(quote! {
                    #peripheral_crate::Interrupt::#dispatcher as u32,
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
        // TODO: this interface can be improved by providing a struct like this:
        /*
        LockParams {
            resource_handle : TokenStream, // &mut resource
            current_task_priority : TokenStream, // u16 value of current task priority
            ceiling_value: TokenStream, // u16 value of resource priority
            lock_closure_handle: TokenStream, // the f(resource: &ResourceType)
        }

        // Then implementation can look like this
        quote! {
            unsafe {rtic::export::lock(#resource_handle,
                                       #current_task_priority,
                                       #ceiling_value,
                                       &__rtic_internal_MASKS, // comes from  `compute_priority_masks(...)` implementation
                                       #lock_closure_handle
                                       );}
        }

        */
        quote! {
            unsafe {rtic::export::lock(resource, task_priority, CEILING, &__rtic_internal_MASKS, f);}
        }
    }
}

#[proc_macro_attribute]
pub fn app(args: TokenStream, input: TokenStream) -> TokenStream {
    RticAppBuilder::new(Rp2040Rtic).parse(args, input)
}
