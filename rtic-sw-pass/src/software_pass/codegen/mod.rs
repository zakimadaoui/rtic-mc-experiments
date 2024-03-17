// [x] generated priority types and queue
// [x] generate an inputs queue for each task
// [x] spawn api for each task
// [x] generated dispatcher tasks
// [ ] export queue type
// [ ] export or generate software task trait, put in exports ?
// [ ] other TODOs in this doc
// [ ] regarding "rtic::export::.." either original crate name is needed or crate_name as rtic needs to be added.
    // something like a trait function called exports_path() inserted into user inputs OR. and in code use rtic_exports::<whatever>::...
    // feels like another convention
// [ ] also need a convention between passes for how task static instances will be named
// for visualization purposes, between each pass save the generated tokenstream to a file.

mod utils;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use crate::software_pass::analyze::AppAnalysis;
use crate::software_pass::parse::ast::SoftwareTask;
use crate::software_pass::parse::{ParsedApp, SWT_TRAIT_TY};

pub fn generate(app: &ParsedApp, analysis: &AppAnalysis) -> TokenStream {

    let sw_tasks =  app.sw_tasks.iter().map(|task|{
        let task_struct = &task.task_struct;
        let task_user_impl = &task.task_impl;
        let spawn_impl = task.generate_spawn_api(analysis);

        quote! {
            #task_struct
            #task_user_impl
            #spawn_impl
        }

    });

    let dispatcher_tasks = generate_dispatcher_tasks(analysis);

    let user_code = &app.rest_of_code;

    //TODO: get sw task trait and include it in the quote or at least put it as part of exports later
    let software_task_trait = format_ident!("{SWT_TRAIT_TY}");
    let sw_task_trait_def = quote! {
        /// Trait for an idle task
        pub trait #software_task_trait {
            type SpawnInput;
            /// Task local variables initialization routine
            fn init() -> Self;
            /// Function to be executing when the scheduled software task is dispatched
            fn exec(&mut self, input: Self::SpawnInput);
        }
    };


    quote! {
        #(#user_code)*
        /// ============================= Software-pass content ====================================
        #(#sw_tasks)*
        #(#dispatcher_tasks)*
        #(#sw_task_trait_def)*
    }
}

/// generates:
/// - an enum type for each group of tasks of the same priority
/// - a ready queue for each group of tasks of the same priority
/// - A dispatcher hw task for each priority level
fn generate_dispatcher_tasks(analysis: &AppAnalysis) -> TokenStream {
    let dispatchers = &analysis.dispatcher_priorities;

    let dispatcher_tasks = analysis.sw_tasks_pgroups.iter().map(|(prio, tasks)| {
        let prio_ty = utils::priority_ty_ident(*prio);

        // generate the branches of the match statement for the dispatcher task
        let dispatch_match_branches = tasks.iter().map(|task_ident| {
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
        let ready_queue_size = tasks.len() + 1; // queue size must always be one more than number of tasks
        let dispatcher_irq_name = dispatchers.get(prio).unwrap(); // safe to unwrap due to guarantees from analysis
        let dispatcher_priority = prio.to_string();
        let dispatcher_task_ty = utils::dispatcher_ident(*prio);

        quote! {
            #[derive(Clone, Copy)]
            #[doc(hidden)]
            pub enum #prio_ty {
                #(#tasks,)*
            }

            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            static mut #ready_queue_name: rtic::export::Queue<#prio_ty, #ready_queue_size> = rtic::export::Queue::new();

            #[doc(hidden)]
            #[task( binds = #dispatcher_irq_name , priority = #dispatcher_priority )]
            pub struct #dispatcher_task_ty;

            impl RticTask for #dispatcher_task_ty {
                fn init() -> Self {
                    // TODO: here you need to generate:
                    // - inits for task queues or any MaybeUnit thing related to software tasks
                    Self
                }

                fn exec(&mut self) {
                    unsafe {
                        let mut ready_consumer = #ready_queue_name.split().1;
                        while let Some(task) = ready_consumer.dequeue() {
                            match task {
                                #(#dispatch_match_branches)*
                            }
                        }
                    }
                }
            }
        }
    });

    quote! {
        #(#dispatcher_tasks)*
    }
}

impl SoftwareTask {
    /// generate the spawn() function for the task
    fn generate_spawn_api(&self, analysis: &AppAnalysis) -> TokenStream {
        let task_name = self.name();
        let task_inputs_queue = utils::sw_task_inputs_ident(task_name);
        let task_trait_name = format_ident!("{}", SWT_TRAIT_TY);
        // get the inputs type. see the RticSwTask trait to understand this and where it comes from.
        let inputs_ty = quote!(<#task_name as #task_trait_name>::SpawnInput);

        let _dispatcher_irq_name = analysis
            .dispatcher_priorities
            .get(&self.params.priority)
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
}