use std::collections::BTreeMap;

use proc_macro2::Span;
use syn::spanned::Spanned;

use crate::ParsedRticApp;
use crate::parser::ast::{HardwareTask, SharedResources, SoftwareTask};

#[derive(Debug)]
pub struct AppAnalysis {
    pub sw_tasks_pgroups: BTreeMap<u16, Vec<syn::Ident>>,
    pub dispatcher_priorities: BTreeMap<u16, syn::Ident>,
    pub used_irqs: Vec<syn::Ident>,
}

impl AppAnalysis {
    pub fn run(app: &ParsedRticApp) -> syn::Result<Self> {
        // hw interrupts bound to hadrware tasks
        let mut used_interrupts: Vec<_> = app
            .hardware_tasks
            .iter()
            .filter_map(|t| t.args.interrupt_handler_name.clone())
            .collect();

        // group sw tasks based on their associated priorities
        let mut sw_tasks_pgroups: BTreeMap<u16, Vec<syn::Ident>> = BTreeMap::new();
        for task in app.software_tasks.iter() {
            let task_prio = task.args.priority;
            if let Some(tasks) = sw_tasks_pgroups.get_mut(&task_prio) {
                tasks.push(task.name().clone());
            } else {
                let _ = sw_tasks_pgroups.insert(task_prio, vec![task.name().clone()]);
            }
        }

        // check if the number of dispatchers meets the number of sw task priority groups
        let n_dispatchers = app.args.dispatchers.len();
        let n_priority_groups = sw_tasks_pgroups.len();
        if n_dispatchers != n_priority_groups {
            return Err(syn::Error::new(
                Span::call_site(),
                format!("Expected {n_priority_groups} dispatchers, but found {n_dispatchers}."),
            ));
        }

        // check if dispatchers are not already used by some hw tasks
        // and add to the list of used interrupts if not already in use
        for dispatcher in app.args.dispatchers.iter() {
            if used_interrupts.contains(dispatcher) {
                if n_dispatchers != n_priority_groups {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        format!(
                            "The dispatcher `{dispatcher}` is already used by a hardware task."
                        ),
                    ));
                }
            } else {
                used_interrupts.push(dispatcher.clone());
            }
        }

        // bind dispatchers to priorities
        let mut dispatchers = app.args.dispatchers.clone();
        let dispatcher_priorities = sw_tasks_pgroups
            .keys()
            .copied()
            .map(|p| (p, dispatchers.pop().unwrap()))
            .collect();

        Ok(Self {
            sw_tasks_pgroups,
            dispatcher_priorities,
            used_irqs: used_interrupts,
        })
    }
}

pub fn update_resource_priorities(
    shared: &mut SharedResources,
    hw_tasks: &Vec<HardwareTask>,
    sw_tasks: &Vec<SoftwareTask>,
) -> syn::Result<()> {
    let iter = hw_tasks.iter().chain(sw_tasks.iter());
    for task in iter {
        let task_priority = task.args.priority;
        for resource_ident in task.args.shared_idents.iter() {
            if let Some(shared_element) = shared.get_field_mut(resource_ident) {
                if shared_element.priority < task_priority {
                    shared_element.priority = task_priority
                }
            } else {
                return Err(syn::Error::new(
                    task.task_struct.span(),
                    format!(
                        "The resource `{resource_ident}` was not found in `{}`",
                        shared.strct.ident
                    ),
                ));
            }
        }
    }
    Ok(())
}
