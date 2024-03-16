use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use crate::analysis::AppAnalysis;
use crate::common::rtic_traits::SWT_TRAIT_TY;
use crate::common::utils;
use crate::parser::ast::SoftwareTask;

/// Generates priority types, ready queues, and dispatcher for each priority level
pub fn generate_sw_tasks_dispatchers(app_analysis: &AppAnalysis) -> TokenStream2 {
    let dispatchers = &app_analysis.dispatcher_priorities;
    // generate priority types
    let prio_ts = app_analysis.sw_tasks_pgroups.iter().map(|(prio, tasks)| {
        let prio_ty = utils::priority_ty_ident(*prio);

        // generate the branches of the match statement for the dispatcher
        let task_dispatch_impl = tasks.iter().map(|task_ident| {
            let task_static_handle = utils::ident_uppercase(task_ident);
            let task_inputs_queue = utils::sw_task_inputs_ident(task_ident);
            let prio_ty = &prio_ty;
            quote!{
                #prio_ty::#task_ident => {
                    let mut input_consumer = #task_inputs_queue.split().1;
                    let input = input_consumer.dequeue_unchecked();
                    #task_static_handle.assume_init_mut().exec(input);
                }
            }
        });

        let ready_queue_name = utils::priority_queue_ident(&prio_ty);
        let queue_size = tasks.len() + 1; // queue size must always be one more than number of tasks
        let dispatcher = dispatchers.get(prio).unwrap();

        quote! {
            #[derive(Clone, Copy)]
            #[doc(hidden)]
            pub enum #prio_ty {
                #(#tasks,)*
            }

            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            static mut #ready_queue_name: rtic::export::Queue<#prio_ty, #queue_size> = rtic::export::Queue::new();

            #[allow(non_snake_case)]
            #[no_mangle]
            fn #dispatcher() {
                unsafe {
                    let mut ready_consumer = #ready_queue_name.split().1;
                    while let Some(task) = ready_consumer.dequeue() {
                        match task {
                            #(#task_dispatch_impl)*
                        }
                    }
                }
            }
        }
    });

    quote! {
        #(#prio_ts)*
    }
}

impl SoftwareTask {
    /// generates inputs queue and spawn implementation for the task
    pub fn generate_sw_task_spawn_api(&self, app_analysis: &AppAnalysis) -> TokenStream2 {
        let task_name = self.name();
        let task_inputs_queue = utils::sw_task_inputs_ident(task_name);
        let task_trait_name = format_ident!("{}", SWT_TRAIT_TY);
        let inputs_ty = quote!(<#task_name as #task_trait_name>::SpawnInput);

        let _dispatcher = app_analysis
            .dispatcher_priorities
            .get(&self.args.priority)
            .unwrap(); // safe to unwrap at this point
        quote! {
            static mut #task_inputs_queue: rtic::export::Queue<#inputs_ty, 2> = rtic::export::Queue::new();

            impl #task_name {
                pub fn spawn(input : #inputs_ty) -> Result<(), #inputs_ty> {
                    // critical section begin
                    // TODO: 1- warning, you can't propagate error in middle, or deadlock, need interrupt::free(|| {})
                    // TODO: 2- its enough to check if inputs queue has place, if so, you can enqueue_uncheched to the priority queue.
                    // critical section end
                    // TODO: pend(), implementor or just export ? i favor first
                    Ok(())
                }
            }
        }
    }

    pub fn generate_sw_task_queue_inits(&self, _app_analysis: &AppAnalysis) -> TokenStream2 {
        todo!()
    }
}
