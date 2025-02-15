use super::*;

/// Interface for providing the low-level hardware bindings specific for a target(s) (A.k.a The Backend) to be used during code generation phase
/// of the **Core Compilation Pass*.
pub trait CorePassBackend {
    /// # Setting up the system
    /// Implementation must return the TokenStream to be inserted **AFTER** the call to Global `#[init]` and tasks init() functions,
    /// and **BEFORE** starting the idle task.
    ///
    /// Note that the generated code resulting from the returned TokenStream will be wrapped in a critical section (interrupts
    /// are disabled at start and re-enabled at end)
    ///
    /// ## Use case
    /// This trait method is meant to cover the following use cases:
    /// - enabling interrupt lines used by the application
    /// - setting priority of interrupts, and similar initializations depending on specific hardware details
    /// - multicore systems where a master core needs to wake-up and initialize other cores (see rp2040 distribution as an example)
    /// ## Note
    /// This function will be called several times in case of a multicore system, each time with different `app_info` and `app_analysis`.
    /// ## Arguments
    /// - `app_args`: arguments provided to the #[app(...)] macro attribute, this includes paths to PACs, number of cores...
    /// - `app_info`: Contains the parsed user application. For single core this will be the full application. For multicore, this represents only a sub-application corresponding to a specific core.
    /// - `app_analysis`: Information about the analyzed application. For single core this will be the analysis of the full application. For multicore, this represents the analysis of a sub-application corresponding to a specific core.
    fn post_init(
        &self,
        app_args: &AppArgs,
        app_info: &SubApp,
        app_analysis: &SubAnalysis,
    ) -> Option<TokenStream2>;

    /// # SRP-based Resource locking implementation
    ///
    /// The provided method argument `incomplete_lock_fn` holds the TokenStream representation of an incomplete function named `lock` responsible for locking a distinct resource in the system. The distribution must generate the missing target-specific logic for implementing the locking of that resource.
    ///
    /// To illustrate this further with an example, let's assume the user defined the following shared resources:
    ///
    /// ```rust
    /// // Before code expansion
    /// #[shared]
    /// struct Shared {
    ///     pub resource1: R1Type
    /// }
    /// ```
    ///
    /// Every field of the shared resources struct has a corresponding autogenerated **resource proxy** struct that implements the `RticMutex` internal trait. as follows:
    ///
    /// ```rust
    /// struct __resource1_mutex {
    ///     #[doc(hidden)]
    ///     task_priority: u16,
    /// }
    /// impl RticMutex for __resource1_mutex {
    ///     type ResourceType = R1Type;
    ///     // this is what the trait method argument `incomplete_lock_fn` expands to
    ///     fn lock(&mut self, f: impl FnOnce(&mut Self::ResourceType)) {
    ///         const CEILING: u16 = 3u16; // resource ceiling
    ///         let task_priority = self.task_priority; // current task priority
    ///         let resource_ref = unsafe { &mut SHARED.assume_init_mut().resource1 };
    ///         /* TODO: THE HARDWARE-SPECIFIC CODE COMES HERE */
    ///     }
    /// }
    /// ```
    ///
    /// Each time RTIC needs to generate an implementation of the `RticMutex` trait for a **resource proxy**, it calls the method [CorePassBackend::generate_resource_proxy_lock_impl] to ask the backend to populate the missing details of the [lock] function for that particular resource.
    ///
    /// ## Contract
    ///
    ///* The returned value representing the populated function must have the same signature as `incomplete_lock_fn'.
    ///
    ///* The implementation must be according to SRP rules such that:
    ///    * System interrupt priority ceiling is raised to the value of `CEILING`.
    ///    * The closure `f` is called and `resource_ref` is passed to it as a parameter. (to execute the resource critical section).
    ///    * System interrupt priority ceiling should be restored back to `task_priority` value.
    ///* If global definitions need to be generated for use in the locking implementation, the trait method which will be described next should be used to cover such need.
    ///
    /// ## Note
    /// This trait method is called for every shared resource in every sub-application.
    ///
    /// ## Debugging Tip
    /// Use ```eprintln("{}", incomplete_lock_fn.to_token_stream().to_string())``` to see the `incomplete_lock_fn` signature and already provided logic inside it.
    fn generate_resource_proxy_lock_impl(
        &self,
        app_args: &AppArgs,
        app_info: &SubApp,
        incomplete_lock_fn: syn::ImplItemFn,
    ) -> syn::ImplItemFn;

    /// # Implementation specific pre-computed values and global definitions
    /// When the Implementation requires pre-computed constants, additional global `use' statements or additional function definitions that must to be accessible from the global application scope, the above trait method could be implemented to return the TokenStream representing those global definitions.
    ///
    /// ## Example Use case
    /// A practical example could be the RTIC implementation for a cortex M0/M0+ based MCU, where locking is implemented using Interrupt priority masking, where the masks are statically computed values accessible from the global application scope. The masks computation can and should use the information provided by `app_args`, `app_info` and `app_analysis` arguments.
    ///
    /// For a real example, see the rp2040 distribution which targets a cortex M0+ based MCU.
    ///
    /// ## Note
    /// This function will be called several times in case of a multicore system, each time with different `app_info` and `app_analysis` for each core.
    ///
    /// ## Arguments
    /// - `app_args`: arguments provided to the #[app(...)] macro attribute, this includes paths to PACs, number of cores...
    /// - `app_info`: Contains the parsed user application. For single core this will be the full application. For multicore, this represents only a sub-application corresponding to a specific core.
    /// - `app_analysis`: Information about the analyzed application. For single core this will be the analysis of the full application. For multicore, this represents the analysis of a sub-application corresponding to a specific core.
    fn generate_global_definitions(
        &self,
        app_args: &AppArgs,
        app_info: &SubApp,
        app_analysis: &SubAnalysis,
    ) -> Option<TokenStream2>;

    /// # Wrapping task execution
    /// In certain cases, some code needs to be executed before and after the [exec] method of a task is called within an interrupt handler. To allow this "wrapping" of the task execution, this trait method can be implemented. The statement from calling the task's [exec] method has been provided as input to this trait method (`dispatch_task_call`), If you return a Some(tokenstream), the returned TokenStream must include/wrap the `dispatch_task_call`.
    ///
    /// ## Example use case
    /// An example could be an implementation for cortex M MCUs that support a BASEPRI register. Every time an interrupt handler id called, the current value of BASEPRI needs to be saved, then task [exec] method is called, then the saved BASEPRI value is restored.
    ///
    /// ## Arguments
    /// - `dispatch_task_call`: call to the task [exec] method.
    ///
    /// ## Contract
    /// The `dispatch_task_call` token stream must be placed in between your custom logic. This tokenstream must not be mutated.
    fn wrap_task_execution(
        &self,
        task_prio: u16, // TODO: more information needs to be provided here to cover more complex cases
        dispatch_task_call: TokenStream2,
    ) -> Option<TokenStream2>;

    /// # Entry name for a specific core
    /// This trait method allows specifying the name of the entry function on each core.
    ///
    /// ## Use case
    /// This trait method is especially useful for multicore applications that result in a single output binary (single-binary systems)
    /// where there are multiple entries (one for each core) but only one entry must to be named `main` while other entries are given
    /// other unique identifiers.
    ///
    /// ## Contract
    /// - For single-core and multi-binary multicore distributions, this trait method should always return "main" as the entry name.
    /// - For single-binary multicore distributions, return "main" once only, then different identifiers should be used for other cores entries.
    ///
    /// See rp2040 distribution for a real world example.
    ///
    /// By default you should implement this function as
    /// ```rust
    /// fn entry_name(&self, core: u32) -> Ident {
    ///     format_ident!("main")
    /// }
    /// ```
    ///
    fn entry_name(&self, core: u32) -> Ident;

    /// # Customizing the default behavior of idle task
    /// When the user doesn't define an idle task, RTIC automatically defines one with a default implementation.
    /// The [exec] method of this default implementation contains an endless loop. This trait method allows customizing
    /// what is executed inside that loop. For instance, the returned TokenStream can be a call to the `wfi` instruction
    /// such that if the idle task is resumed, the device immediately goes to sleep mode and waits for more interrupts to
    /// wake it up.
    ///
    /// # Customizing the default behavior of idle task
    /// When the user doesn't define an idle task, RTIC automatically defines one with a default implementation.
    /// The [exec] method of this default implementation contains an endless loop. This trait method allows customizing
    /// what is executed inside that loop. For instance, the returned TokenStream can be a call to the `wfi` instruction
    /// such that if the idle task is resumed, the device immediately goes to sleep mode and waits for more interrupts to
    /// wake it up.
    ///
    /// If None is returned, the default implementation of the [exec] method of the idle task will be an empty infinite loop
    /// that will only waste cycles !.
    fn populate_idle_loop(&self) -> Option<TokenStream2>;

    /// # Non-preemptable code sections
    /// The RTIC implementation occasionally generates code that must run in a non-preemptable fashion. Therefore, a
    /// distribution must provide the implementation of this trait method to populate the body of `empty_body_fn` with
    /// the low-level implementation required to generate the function to be used to execute non-interruptible code
    /// (i.e a traditional critical sections)
    ///
    /// The `empty_body_fn` argument, is a token stream for a function that expands to the following:
    /// ```rust
    /// #[inline]
    /// pub fn __rtic_critical_section<F, R>(f: F) -> R
    /// where F: FnOnce() -> R,
    /// {
    ///    /* TODO: You need to fill this part here */
    /// }
    /// ```
    ///
    /// ## Contract
    /// - You MUST not change the function signature (of `empty_body_fn`)
    /// - The generated function must re-enable interrupts at end of the critical section.
    fn generate_interrupt_free_fn(&self, empty_body_fn: syn::ItemFn) -> syn::ItemFn;

    /// # The "multibin" feature additional requirements
    /// If a distribution enables `multibin` feature to allow targeting a multi-binary target, then the distibution must:
    /// - re-export microamp crate
    /// - implement this trait method to provide  te path to the re-exported `shared` attribute macro from microamp crate.
    ///
    /// Example implementation can be
    /// ```rust
    /// fn multibin_shared_macro_path() -> syn::Path {
    ///     syn::parse_quote! { rtic::export::microamp::shared }
    /// }
    ///
    /// This will be used by RTIC internally to generate the statement:
    /// ```rust
    /// use rtic::export::microamp::shared as multibin_shared;
    ///
    /// where multibin_shared is the proc macro attribute used to indicate shared data across cores
    /// ```
    #[cfg(feature = "multibin")]
    fn multibin_shared_macro_path(&self) -> syn::Path;

    /// # Additional user code validation
    /// Implement this method to validate/analyze the resulting parsed and analyzed user application before the code generation phase starts.
    ///
    /// ## Use case
    /// In certain cases, some checks/validation related to implementation/hardware specific details need to be made before allowing the user code to be expanded.
    /// An example, could be that the user has attempted to use an Exception line as for a dispatcher, but the distribution needs to forbid that.
    /// Implementing this trait method, gives the ability to enforcing such checks.
    fn pre_codegen_validation(&self, app: &App, analysis: &Analysis) -> syn::Result<()>;

    /// Implementation must return the default task priority to be used in idle task and tasks when priority argument value is not provided by the user.
    fn default_task_priority(&self) -> u16;
}
