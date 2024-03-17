use std::collections::BTreeMap;

use proc_macro2::Span;
use crate::parse::ParsedApp;

#[derive(Debug)]
pub struct AppAnalysis {
    pub sw_tasks_pgroups: BTreeMap<u16, Vec<syn::Ident>>,
    pub dispatcher_priorities: BTreeMap<u16, syn::Ident>,
}

impl AppAnalysis {
    pub fn run(app: &ParsedApp) -> syn::Result<Self> {

        // group sw tasks based on their associated priorities
        let mut sw_tasks_pgroups: BTreeMap<u16, Vec<syn::Ident>> = BTreeMap::new();
        for task in app.sw_tasks.iter() {
            let task_prio = task.params.priority;
            if let Some(tasks) = sw_tasks_pgroups.get_mut(&task_prio) {
                tasks.push(task.name().clone());
            } else {
                let _ = sw_tasks_pgroups.insert(task_prio, vec![task.name().clone()]);
            }
        }

        // check if the number of dispatchers meets the number of sw task priority groups
        let n_dispatchers = app.app_params.dispatchers.len();
        let n_priority_groups = sw_tasks_pgroups.len();
        if n_dispatchers != n_priority_groups {
            return Err(syn::Error::new(
                Span::call_site(),
                format!("Expected {n_priority_groups} dispatchers, but found {n_dispatchers}."),
            ));
        }

        // bind dispatchers to priorities
        let mut dispatchers = app.app_params.dispatchers.clone();
        let dispatcher_priorities = sw_tasks_pgroups
            .keys()
            .copied()
            .map(|p| (p, dispatchers.pop().unwrap()))
            .collect();

        Ok(Self {
            sw_tasks_pgroups,
            dispatcher_priorities,
        })
    }
}

