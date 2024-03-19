mod utils;

use crate::software_pass::analyze::AppAnalysis;
use crate::software_pass::parse::ast::SoftwareTask;
use crate::software_pass::parse::{ParsedApp, SWT_TRAIT_TY};
use crate::ScSoftwarePassImpl;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemFn, parse_quote};

pub struct CodeGen<'a> {
    app: &'a ParsedApp,
    analysis: &'a AppAnalysis,
    implementation: &'a dyn ScSoftwarePassImpl,
}

impl<'a> CodeGen<'a> {
    pub fn new(
        app: &'a ParsedApp,
        analysis: &'a AppAnalysis,
        implementation: &'a dyn ScSoftwarePassImpl,
    ) -> CodeGen<'a> {
        Self {
            app,
            analysis,
            implementation,
        }
    }

    pub fn run(&self) -> TokenStream {
        let app = self.app;
        let analysis = self.analysis;
        let sw_tasks = app.sw_tasks.iter().map(|task| {
            let task_struct = &task.task_struct;
            let task_user_impl = &task.task_impl;
            let spawn_impl = task.generate_spawn_api(app, analysis);

            quote! {
                #task_struct
                #task_user_impl
                #spawn_impl
            }
        });

        let dispatcher_tasks = self.generate_dispatcher_tasks();

        let pend_fn_def = self.get_pend_fn();

        let user_code = &app.rest_of_code;

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

        let mod_visibility = &app.mod_visibility;
        let mod_ident = &app.mod_ident;

        quote! {
            #mod_visibility mod #mod_ident {
                #(#user_code)*
                /// ============================= Software-pass content ====================================
                #(#sw_tasks)*
                #dispatcher_tasks
                #sw_task_trait_def
                #pend_fn_def
            }
        }
    }

    /// generates:
    /// - an enum type for each group of tasks of the same priority
    /// - a ready queue for each group of tasks of the same priority
    /// - A dispatcher hw task for each priority level
    fn generate_dispatcher_tasks(&self) -> TokenStream {
        let analysis = self.analysis;
        let dispatchers = &analysis.dispatcher_priorities;

        let dispatcher_tasks = analysis.sw_tasks_pgroups.iter().map(|(prio, tasks)| {
            let prio_ty = utils::priority_ty_ident(*prio);

            // generate the branches of the match statement for the dispatcher task
            let dispatch_match_branches = tasks.iter().map(|task_ident| {
                let task_static_handle = utils::ident_uppercase(task_ident);
                let task_inputs_queue = utils::sw_task_inputs_ident(task_ident);
                let prio_ty = &prio_ty;
                quote! {
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
            let dispatcher_priority = prio;
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

    fn get_pend_fn(&self) -> ItemFn {
        let pend_fn_ident = format_ident!("{PEND_FN_NAME}");
        let pend_fn_empty = parse_quote! {
            #[doc(hidden)]
            #[inline]
            pub fn #pend_fn_ident(irq_nbr : u16) {
                // To be implemented by distributor
                // example:
                // let irq : pac::Interrupt = unsafe { core::mem::transmute(irq_nbr) }
                // NVIC::pend( irq );
            }
        };
        self.implementation.fill_pend_fn(pend_fn_empty)
    }
}

pub const PEND_FN_NAME: &str = "__rtic_sc_pend";

impl SoftwareTask {
    /// generate the spawn() function for the task
    fn generate_spawn_api(&self, app: &ParsedApp, analysis: &AppAnalysis) -> TokenStream {
        let task_name = self.name();
        let task_inputs_queue = utils::sw_task_inputs_ident(task_name);
        let task_trait_name = format_ident!("{}", SWT_TRAIT_TY);
        // get the inputs type. see the RticSwTask trait to understand this and where it comes from.
        let inputs_ty = quote!(<#task_name as #task_trait_name>::SpawnInput);
        let user_peripheral_crate = &app.app_params.device;
        let prio_ty = utils::priority_ty_ident(self.params.priority);
        let ready_queue_name = utils::priority_queue_ident(&prio_ty);

        let dispatcher_irq_name = analysis
            .dispatcher_priorities
            .get(&self.params.priority)
            .unwrap(); // safe to unwrap at this point

        let pend_fn = format_ident!("{PEND_FN_NAME}");
        let critical_section_fn = format_ident!("{}", rtic_core::rtic_functions::INTERRUPT_FREE_FN);

        quote! {
            static mut #task_inputs_queue: rtic::export::Queue<#inputs_ty, 2> = rtic::export::Queue::new();

            impl #task_name {
                pub fn spawn(input : #inputs_ty) -> Result<(), #inputs_ty> {
                    let mut inputs_producer = unsafe {#task_inputs_queue.split().0};
                    let mut ready_producer = unsafe {#ready_queue_name.split().0};
                    /// need to protect by a critical section due to many producers of different priorities can spawn/enqueue this task
                    #critical_section_fn(|| -> Result<(), #inputs_ty>  {
                        // enqueue inputs
                        inputs_producer.enqueue(input)?;
                        // enqueue task to ready queue
                        unsafe {ready_producer.enqueue_unchecked(#prio_ty::#task_name)};
                        // pend dispatcher
                        #pend_fn(#user_peripheral_crate::Interrupt::#dispatcher_irq_name as u16);
                        Ok(())
                    })
                }
            }
        }
    }
}
