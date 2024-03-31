use syn::spanned::Spanned;

use crate::parser::ast::{HardwareTask, SharedResources};
use crate::parser::RticSubApp;

#[derive(Debug)]
pub struct AppAnalysis {
    // used interrupts and their priorities
    pub used_irqs: Vec<(syn::Ident, u16)>,
}

impl AppAnalysis {
    pub fn run(app: &RticSubApp) -> syn::Result<Self> {
        // hw interrupts bound to hardware tasks
        let used_interrupts = app
            .tasks
            .iter()
            .filter_map(|t| Some((t.args.interrupt_handler_name.clone()?, t.args.priority)))
            .collect();

        Ok(Self {
            used_irqs: used_interrupts,
        })
    }
}

pub fn update_resource_priorities(
    shared: Option<&mut SharedResources>,
    hw_tasks: &[HardwareTask],
) -> syn::Result<()> {
    let Some(shared) = shared else { return Ok(()) };
    for task in hw_tasks.iter() {
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
