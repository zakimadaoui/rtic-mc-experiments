use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::Ident;

use crate::parser::ast::{HardwareTask, SharedResources};
use crate::parser::SubApp;
use crate::App;
use heck::ToSnakeCase;
pub struct Analysis {
    pub sub_analysis: Vec<SubAnalysis>,
}

impl Analysis {
    pub fn run(app: &App) -> syn::Result<Self> {
        let sub_analysis = app
            .sub_apps
            .iter()
            .map(SubAnalysis::run)
            .collect::<syn::Result<_>>()?;
        Ok(Self { sub_analysis })
    }
}

#[derive(Debug)]
pub struct SubAnalysis {
    // used interrupts and their priorities
    pub used_irqs: Vec<(syn::Ident, u16)>,
    // tasks requiring some late local resource initialization.
    pub late_resource_tasks: Vec<LateResourceTask>,
}

impl SubAnalysis {
    pub fn run(app: &SubApp) -> syn::Result<Self> {
        // hw interrupts bound to hardware tasks
        let used_interrupts = app
            .tasks
            .iter()
            .filter_map(|t| Some((t.args.interrupt_handler_name.clone()?, t.args.priority)))
            .collect();

        let user_initializable_tasks = app
            .tasks
            .iter()
            .chain(app.idle.iter()) // idle is also a task and we shouldn't forget it
            .filter_map(|t| {
                if t.user_initializable {
                    Some(LateResourceTask {
                        task_name: t.task_struct.ident.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(Self {
            used_irqs: used_interrupts,
            late_resource_tasks: user_initializable_tasks,
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

#[derive(Debug)]
pub struct LateResourceTask {
    pub task_name: Ident,
}
impl LateResourceTask {
    /// By convention, this method is used to generate the name of the static task instance
    pub fn name_uppercase(&self) -> Ident {
        let name = self.task_name.to_string().to_snake_case().to_uppercase();
        Ident::new(&name, Span::call_site())
    }

    pub fn name_snakecase(&self) -> Ident {
        let name = self.task_name.to_string().to_snake_case();
        Ident::new(&name, Span::call_site())
    }
}
