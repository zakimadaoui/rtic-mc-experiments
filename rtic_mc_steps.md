The question that i have been asking for a long while before starting this change was: should there be multi-core pass ? or should the standard pass and software pass be re-factored to be generic enought to fit multicore applications. And after some experimenation, the answer was neither nor, but a bit of both !. The main reasons were:
- to have a multi-core application with multiple entries, the standard-pass must be able to detect that and accomodate for that.
- multi-core software tasks implementation are 98% similar to core-local task with the only exception beign the cross_pend() instead of pend()
- it is easier to distribute tasks to dispatchers and make analysis and validation when information about core-local and cross-core software tasks is available in the same pass/crate

In addition to re-factoring the standard-pass and software-pass to allow generic multi-core applications. Some extra compilation passes will be added to improve multi-core support such as adding a lightweight pass to perform automatic task assignment to cores. and maybe another pass to allow producing multiple binaries from a single source (for multi-core architectures that require this)


Initilly the `rtic-core (standard pass)` and `rtic-sw-pass (software pass)` crates were written for single core rtic applications. But now they have been refactored to be more flexible and allow more generic N-core applicatations. The refactoring was done to mimic what should be done for the real RTIC 2.0 framework and to measure the complexity of such changes.

The re-factoring was much simpler than i expected, only few structural changes and few extra trait functions were need. And that's what i will be describing soon...

### Adding two attributes to task arguments 
The `#[task]`, `#[sw_task]`, `#[init]` and `#[idle]` attributes accept one more agument `core = N` for static assignment to a specific Core with the default value of 0 (i.e no observalble change for single core applications).

In addition the `#[sw_task]` accepts one further agument which is `spawn_by = N` argument to specify which Core is allowed to spawn this software task. And, `spawn_by = N` has a default value of `spawn_by = core`


### Changing the output structure of the application parsing phase
Initially the parsing phase used to output:
```rust
pub struct App {
    pub app_name: Ident,
    pub args: AppArgs,
    pub shared: SharedResources,
    pub init: InitTask,
    pub idle: Option<IdleTask>,
    pub hardware_tasks: Vec<RticTask>,
    pub user_includes: Vec<ItemUse>,
    pub other_code: Vec<Item>,
}
```

and now based on the  `core = N` filtering, it has been changed to: 
```rust 
/// Type to represent a sub application (application on a single core)
#[derive(Debug)]
pub struct SubApp {
    pub core: u32,
    pub shared: Option<SharedResources>,
    pub init: InitTask,
    pub idle: Option<IdleTask>,
    pub tasks: Vec<HardwareTask>,
}

/// Type to represent an RTIC application (withing software pass context)
/// The application contains one or more sub-applications (one application per-core)
#[derive(Debug)]
pub struct App {
    pub app_name: Ident,
    pub args: AppArgs,
    /// a list of sub-applications, one sub-app per core.
    pub sub_apps: Vec<SubApp>,
    pub user_includes: Vec<ItemUse>,
    pub other_code: Vec<Item>,
}
```

With this change in place, most of the existing code was reused to iteratively go over each `SubApp` instead of going once though one `App`. And similary for the analysis phase. Instead of generating one `Analysis` struct corresponding for `App`. Now, we have a list of `SubAnalysis` structs, each corresponding to one `SubApp`.


Similarly in the software pass crate the following structural changes were made by filtering elements based on `core = M` and `spawn_by = N`:


from
```rust
pub struct App {
    pub mod_visibility: Visibility,
    pub mod_ident: Ident,
    pub app_params: AppParameters,
    pub sw_tasks: Vec<SoftwareTask>,
    pub rest_of_code: Vec<Item>,
}
```

To

```rust
/// Type to represent a sub application (application on a single core)
pub struct SubApp {
    pub core: u32,
    pub dispatchers: Vec<syn::Path>,
    /// Single core/ Core-local software tasks
    pub sw_tasks: Vec<SoftwareTask>,
    /// Multi core/ software tasks to be spawned on this core from other cores
    pub mc_sw_tasks: Vec<SoftwareTask>,
}

/// Type to represent an RTIC application (withing software pass context)
/// The application contains one or more sub-applications (one application per-core)
pub struct App {
    pub mod_visibility: Visibility,
    pub mod_ident: Ident,
    pub app_params: AppParameters,
    /// a list of sub-applications, one sub-app per core.
    pub sub_apps: Vec<SubApp>,
    pub rest_of_code: Vec<Item>,
}
```

### Changes in rtic-core standard pass code generation
One main change in the code generation of the standard pass is the fact that the generated application can have multiple entries instead of one. I.e, one entry for each `SubApplication`. In addition, the name of the generated entry is left to be defined by the distribution thought impelmenting the `entry_name(&self, core: u32) -> Ident` function for the `StandardPassImpl` trait. This allows for example to have one entry called `main` and other entries having custom names which can later be used in initializing the other core.


### Changes in analysis and validation
The software pass analysis was updated to enforce the following rules:
- If core A spawns a task on core B, the same task cannot be spawned from core B. Similarly if core A spawns a task locally, the same task cannot be spawned by another core. This decision is there to avoid a race condition across cores when inserting a ready task to some ready tasks priority queue during the call to `spawn()`. 
- Due to the previous condition, A Dispatcher will be **reserved** entirely for either 
  - tasks that are spawned locally (local core message passing)
  - tasks that are spawned by the other core (cross-core message passing in One Direction)
  - and it is **forbidden** to have a dispatcher that sevrves both the above purposes due to race conditions
- In addition, cross-core tasks of the same priority group must all have the same spawn_by index. I,e a dispatcher for cross core tasks can only serve one SPAWNER core.
- Priority of cross-core tasks cannot overlap with the priority of core-local tasks (they must have different dipatchers)


In addition, now a unique type for each core is generated as follows:
```rust
pub use core_1_type::Core1;
mod core_1_type {
    struct Core1Inner;
    pub struct Core1(Core1Inner);
    impl Core1 {
        pub const unsafe fn new() -> Self {
            Core1(Core1Inner)
        }
    }
}
```

This type is made intentionally unsafe to instantiate to make sure the user doen't use this or atleast taint the application with unsafe code. This type is used internally to guarantee that multi-core tasks can be spawned only from a specific core specified by the `spawn_by = N` argument.


### Additional trait function for cross pending
In addition to the above changes to the software pass, an additional trait function was added to `SoftwarePassImpl` to allow binding an external cross core pending logic/function:

```rust
/// Interface for providing the hardware specific details needed by the software pass
pub trait SoftwarePassImpl {
    /// Provide the implementation/body of the core local interrupt pending function. (implementation is hardware dependent)
    /// You can use [eprintln()] to see the `empty_body_fn` function signature
    fn impl_pend_fn(&self, empty_body_fn: syn::ItemFn) -> syn::ItemFn;

    /// (Optionally) Provide the implementation/body of the cross-core interrupt pending function. (implementation is hardware dependent)
    /// You can use [eprintln()] to see the `empty_body_fn` function signature
    fn impl_cross_pend_fn(&self, empty_body_fn: syn::ItemFn) -> Option<syn::ItemFn>;
}
```

and in the core generation part, the framework desides which pend function to use based on `spawn_by` vs `core` values for each software task.




Other miscelanious changes haven't been mentioed yet and will be added here later...