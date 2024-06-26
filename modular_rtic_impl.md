# Modular RTIC Implementation details 

To be able to decouple the hardware details from the declaratic model parsing and generation the `rtic-core (standard pass)` and other `compilation pass crates` expose a set of traits that can be implemented at the **distribution crate** level. The **distribution crate** aids the generic code generation part by suppling the missing hardware specific details though implementing those traits and dynamically passing the implementation.  

### Traits in `rtic-core`:

#### StandardPassImpl

This trait acts as an interface between the `distribution crate`  and  `rtic-core` to provide hardware/architecture specific details needed during the code generation of the *hardware tasks and resources pass*.

```rust

/// Interface for providing hw/architecture specific details for implementing the standard tasks and resources pass both
/// for single core and multicore systems.
pub trait StandardPassImpl {
    /// Returns the default task priority to be used in idle task and tasks where priority argument is not mentioned
    fn default_task_priority(&self) -> u16;

    /// Return the code to be inserted AFTER the call to Global #[init] annotated function and task specific init() functions, and BEFORE starting the idle task.
    /// ## Use case
    /// This trait method is meant to cover the following use cases:
    /// - enabling interrupt lines used by the application
    /// - setting priority of interrupts, and similar initializations depending on specific hardware details
    /// - multicore systems where a master core needs to makeup and initialize other cores (see rp2040 distribution as an example)
    /// ## Note
    /// This function will be called several times in case of a multicore system, each time with different `app_info` and `app_analysis`.
    /// ## Arguments
    /// - `app_args`: arguments provided to the #[app(...)] macro attribute, this includes paths to PACs, number of cores...
    /// - `app_info`: Contains the parsed user application. For single core this will be the full application.
    /// For multicore, this represents only a sub-application corresponding to a specific core.
    /// - `app_analysis`: Information about the analyzed application. For single core this will be the analysis of the full application.
    /// For multicore, this represents the analysis of a sub-application corresponding to a specific core.
    fn post_init(
        &self,
        app_args: &AppArgs,
        app_info: &SubApp,
        app_analysis: &SubAnalysis,
    ) -> Option<TokenStream2>;

    /// LOCKING PART I: first utility for generating code to be used for implementing the locking mechanism of resource proxies.
    ///
    /// See [StandardPassImpl::impl_resource_proxy_lock] documentation for understanding what resource proxies are.
    ///
    /// ## Use case
    /// This trait method can be used for generating code of static variables, `use` statements or any other global declarations
    /// to be used for implementing the locking mechanism of a resource proxy.
    /// An example use case could be a cortex M0/M0+ based MCU, where locking is implemented using Interrupt priority masking,
    /// and the masks are computed statically. The masks computation can and should use the information provided by
    /// `app_args`, `app_info` and `app_analysis` arguments.
    ///
    /// For a real example, see the rp2040 distribution.
    ///
    /// ## Note
    /// This function will be called several times in case of a multicore system, each time with different `app_info` and `app_analysis`.
    ///
    /// ## Arguments
    /// - `app_args`: arguments provided to the #[app(...)] macro attribute, this includes paths to PACs, number of cores...
    /// - `app_info`: Contains the parsed user application. For single core this will be the full application.
    /// For multicore, this represents only a sub-application corresponding to a specific core.
    /// - `app_analysis`: Information about the analyzed application. For single core this will be the analysis of the full application.
    /// For multicore, this represents the analysis of a sub-application corresponding to a specific core.
    fn compute_lock_static_args(
        &self,
        app_args: &AppArgs,
        app_info: &SubApp,
        app_analysis: &SubAnalysis,
    ) -> Option<TokenStream2>;

    /// LOCKING PART II: second utility for generating code to be used for implementing the locking mechanism of resource proxies.
    ///
    /// ## Resource proxies
    /// Every shared resource element in the struct annotated with `#[shared]` attribute has a corresponding
    /// autogenerated resource proxy struct that looks like follows:
    ///
    /// ```rust
    /// struct __resource1_mutex {
    ///     #[doc(hidden)]
    ///     task_priority: u16,
    /// }
    /// impl RticMutex for __resource1_mutex {
    ///     type ResourceType = Resource1;
    ///     // this is what the trait method argument `incomplete_lock_fn` expands to
    ///     fn lock(&mut self, f: impl FnOnce(&mut Self::ResourceType)) {
    ///         const CEILING: u16 = 3u16;
    ///         let task_priority = self.task_priority;
    ///         let resource_ptr = unsafe { &mut SHARED_RESOURCES.assume_init_mut().resource1 as *mut _ };
    ///         /* YOUR HARDWARE DEPENDENT CODE COMES HERE FOR IMPLEMENTING LOCKING */
    ///     }
    /// }
    /// ```
    ///
    /// ## Note
    /// This trait method is called for every shared resource in every sub-application.
    ///
    /// ## Contract and Usage
    /// A distribution must complete the implementation of the lock function (see above) for every resource proxy.
    /// The function signature and part of the function body are already given by the `incomplete_lock_fn` argument.
    /// - You MUST not change the function signature
    /// - You SHOULD (but not a must) use the first 3 provided statements in the incomplete function body.
    ///
    /// ## Debugging Tip
    /// Use `eprintln("{}", incomplete_lock_fn.to_tokenstream().to_string())` to see the `incomplete_lock_fn` signature and already provided logic inside it.
    fn impl_resource_proxy_lock(
        &self,
        app_args: &AppArgs,
        app_info: &SubApp,
        incomplete_lock_fn: syn::ImplItemFn,
    ) -> syn::ImplItemFn;

    /// Optionally customize what happens before and after a task [exec()] method is called when its corresponding interrupt is triggered.
    ///
    /// ## Use case
    /// Some systems need to run a custom logic right at the start when an task interrupt is triggered and also at the very end
    /// after the task [exec()] method is called. An example could be cortex M MCUs that have a BASEPRI register. Every time an
    /// interrupt executes, the current value of BASEPRI needs to be saved, then task [exec()] method is called,
    /// then the saved BASEPRI value is restored.
    ///
    /// ## Arguments
    /// - `dispatch_task_call`: call to the task [exec()] method.
    ///
    /// ## Contract
    /// The `dispatch_task_call` token stream must be placed in between your custom logic.
    fn custom_task_dispatch(
        &self,
        task_prio: u16, // TODO: more information needs to be provided here to cover more complex cases
        dispatch_task_call: TokenStream2,
    ) -> Option<TokenStream2>;

    /// Entry name for a specific core
    /// This function is especially useful for multicore applications that result in a single output binary
    /// where there are multiple entries (one for each core) but only one entry needs to be named `main`.
    ///
    /// See rp2040 distribution for an example.
    ///
    /// By default you should implement this function as
    /// ```rust
    /// fn entry_name(&self, core: u32) -> Ident {
    ///     format_ident!("main")
    /// }
    /// ```
    ///
    fn entry_name(&self, core: u32) -> Ident;

    /// Optionally provide a call to WFI (Wait for interrupt) instruction to be used in the body of the default idle task.
    /// If none is returned, and the user doesn't define an idle task, then, a default idle task will be generated with
    /// an infinite loop that has an empty body (this will waste a lot of cycles !).
    ///
    /// ## Tip
    /// You can also use this function to return another instruction or even code different from `wfi` call
    /// that can be used to populate the body of the loop inside the default idle function.
    fn wfi(&self) -> Option<TokenStream2>;

    /// Provide the implementation/body of the critical section function implementation to be used internally
    /// by RTIC, when generated code needs to be executed a critical section
    ///
    /// The `empty_body_fn` argument, is a token stream for a function that expands to the following:
    /// ```rust
    /// #[inline]
    /// pub fn __rtic_critical_section<F, R>(f: F) -> R
    /// where F: FnOnce() -> R,
    /// {
    ///    /* You need to fill this part here */
    /// }
    /// ```
    ///
    /// ## Contract and Usage
    /// A distribution must complete the implementation of the __rtic_critical_section() function (see above).
    /// The function signature the function with an empty body are already given by the `empty_body_fn` argument.
    /// - You MUST not change the function signature
    fn impl_interrupt_free_fn(&self, empty_body_fn: syn::ItemFn) -> syn::ItemFn;

    /// Provide the path to the re-exported microamp::shared macro attribute
    /// Example implementation can be
    /// ```rust
    /// fn multibin_shared_macro_path() -> syn::Path {
    ///     syn::parse_quote! { rtic::exports::microamp::shared}
    /// }
    ///
    /// This will be used to generate the use statement:
    /// ```rust
    /// use rtic::exports::microamp::shared as multibin_shared;
    ///
    /// where multibin_shared is the proc macro attribute used to indicate shared data across cores
    /// ```
    #[cfg(feature = "multibin")]
    fn multibin_shared_macro_path(&self) -> syn::Path;

    /// Implement this method to make validate the resulting parsed and analyzed user application (full application view in both single and multi core systems)
    /// before code generation phase is starts.
    ///
    /// ## Use case
    /// In certain cases, some checks/validation related to implementation/hardware specific details need to be made before allowing the user code to expand. An example, could be that
    /// The user has used an Exception Interrupt as a dispatcher, but the distribution needs to forbid that. Implementing this trait method, gives you
    /// the ability to make such checks.
    fn pre_codgen_validation(
        &self,
        _app_args: &AppArgs,
        _app: &App,
        _analysis: &Analysis,
    ) -> syn::Result<()> {
        Ok(())
    }
}
```

----
### WARNING: ANYTHING AFTER THIS TEXT IS SLIGHTLY OUTDATED

----

##### Abstract RTIC compilation pass trait (RticPass)

This trait must be implemented by every crate that provides a **Compilation pass**. 

```rust
pub trait RticPass {
    fn run_pass(&self, params: RticAttr, app_mod: TokenStream2) -> syn::Result<TokenStream2>;
}
```



##### Application Builder-like API

To bind the compilation passes and hardware specific implementations to `rtic-core`, it provides this following API:

```rust
#[proc_macro_attribute]
pub fn app(args: TokenStream, input: TokenStream) -> TokenStream {
    
    let hw_pass = /*Initialize a struct that implements ScHwPassImpl Trait */;
    
    let sw_pass =  /*Initialize software pass struct from the rtic-sw-pass crate */;
    let mono_pass =  /*Initialize monotonics pass struct from the Monotonics pass crate */;
    let auto_assign_pass =  /* Initialize struct from the automatic task assignment to cores pass crate */;
    let other_pass1 =  /* Initialize pass from 3rd party crate */;
    let other_pass2 =  /* Initialize pass from 3rd party crate  */;
    
    let mut builder = RticAppBuilder::new(hw_pass); // Hardware pass is mandatory, other passes aren't
    builder.add_compilation_pass(CompilationPass::SwPass(sw_pass));
    builder.add_compilation_pass(CompilationPass::MonotonicsPass(mono_pass));
    builder.add_compilation_pass(CompilationPass::McPass(auto_assign_pass));
    builder.add_compilation_pass(CompilationPass::Other(other_pass1));
    builder.add_compilation_pass(CompilationPass::Other(other_pass2));
    builder.build_rtic_application(args, input)

    // passes will be executed in this order
    // - auto_assign_pass
    // - other_pass1
    // - other_pass2
    // - mono_pass
    // - sw_pass
    // - hw_pass
}
```



#### `rtic-sw-pass` interface

The `rtic-sw-pass` crate provides the software tasks pass thought the type `SoftwarePass` which already implements `RticPass` trait. However, missing hardware specific implementation for pend() and cross_pend() functions have to be provided by the distribution through implementing the `SoftwarePassImpl` trait:


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

----

### Putting it all together: A rp2040 multi-core RTIC application implementation

To demonstrate how easy it is to create RTIC distributions, This example will show how an RP2040 multi-core rtic distribution can be built. 

First, let's start by providing the hadware specific details for the Hadware tasks and resources pass by implementing the `ScHwPassImpl` trait:

```rust
impl StandartPassImpl for Rp2040HwPassBackend {
    /* see rp2040-rtic/rp2040-rtic-macro/src/lib.rs and rp2040-rtic/src/export.rs */
    /* Only about 200 lines of code needed */
}
```



Then lets do the same for software pass:

```rust
struct SwPassBackend;
impl SoftwarePassImpl for SwPassBackend {
    fn fill_pend_fn(&self, mut empty_body_fn: ItemFn) -> ItemFn {
        let body = parse_quote!({
            // taken from cortex-m implementation
            unsafe {
                (*rtic::export::NVIC::PTR).ispr[usize::from(irq_nbr / 32)]
                    .write(1 << (irq_nbr % 32))
            }
        });
        empty_body_fn.block = Box::new(body);
        empty_body_fn
    }

    fn impl_cross_pend_fn(&self, mut empty_body_fn: ItemFn) -> Option<ItemFn> {
        let body = parse_quote!({
            rtic::export::cross_core::pend_irq(irq_nbr);
        });
        empty_body_fn.block = Box::new(body);
        Some(empty_body_fn)
    }
}
```



Finally lets bind everything together

```rust
#[proc_macro_attribute]
pub fn app(args: TokenStream, input: TokenStream) -> TokenStream {
    // use the standard software pass provided by rtic-sw-pass crate
    let sw_pass = Box::new(ScSoftwarePass::new(SwPassBackend));

    let mut builder = RticAppBuilder::new(Rp2040HwPassBackend);
    #[cfg(feature = "sw_tasks")] // software tasks pass to be explicitly enabled by the user 
    builder.add_compilation_pass(CompilationPass::SwPass(sw_pass));
    builder.build_rtic_application(args, input)
}
```

----

### Using the resulting Rp2040 RTIC Framework for a Single-core app

```rust
#[rp2040_rtic::app(
    device=rp2040_hal::pac,
    dispatchers= [TIMER_IRQ_2]),
]
pub mod my_single_core_app {

    // user includes ...
    
    #[shared]
    struct SharedResources {
        shared1 : Type,
        shared2 : Type,
    }

    #[init]
    fn init() -> SharedResources {
        // init code ....
        SharedResources { 
        	shared1 : val,
            shared2 : val,
        }
    }

    #[task(binds = TIMER_IRQ_0 , priority = 3, shared = [shared1, shared2])]
    struct MyHwTask {
        /* local resources */
        local_resource1 : bool,
    }
    impl RticTask for MyHwTask {
        fn init() -> Self {
            Self { /* init local resources */ }
        }

        fn exec(&mut self) {
            
            // locking a resource
            self.shared().shared2.lock(|shared2| {
                //do something with `shared2` in a critical section
            });
            
            // using a local resource
            if  self.local_resource1 {
                // do something
                 self.local_resource1 = false
            } else {
                // do something else
            }
            
            // spawning a software task
            if let Err(val) =  MySwTask::spawn(7) {
                // do something on error case,
            }
            
        }
    }
        
    #[sw_task(priority = 1, shared = [shared2])]
    struct MySwTask {
        /* local resources */
    }
    impl RticSwTask for MySwTask {
        type TaskInputType = u8;
        
        fn init() -> Self {
            Self { /* init local resources */ }
        }

        fn exec(&mut self, input: u8) {
            // do something with input
            // lock `shared2` resource 
            // use your creativity
        }
    }
    
    #[idle(shared = [shared1])]
    struct MyIdleTask {
        /* local resources */
    }
    impl RticIdleTask for MyIdleTask {
        fn init() -> Self {
            Self { /* init local resources */ }
        }

        fn exec(&mut self) -> ! {
            loop { /* ... */ }
        }
    }

    
}
```

in **Cargo.toml** of this application the feature `sw_tasks` must be enabled to allow the use of the `dispatchers = [...]` attribute and to allow declaring software tasks. 



**NOTE1:** Because this is just an experiment, `rtic-core` implements only a subset of the Original RTIC declarative model. Currently only Init, idle, Hardware task, Software tasks with message passing, local and shared resources are provided ( monotonics, and async not implemented yet)

**NOTE about changed syntax:** the way of **declaring a task now uses a struct that implements a specific task trait instead of a function** with Context,  the way of **defining local resources is by defining member variables of the Task struct**. and finally, shared resources are accessed through the `shared()` API, instead of using Context. The reason for this change is purely because this allows rapid prototyping due to the fact that it is much easier/faster to implement such model (and IMHO also feels less complicated/more intuitive for first time users, but this is not the purpose of this project and should be skipped for real rtic ).



For a real example on the rp2040 see [hello_rtic.rs](rp2040-rtic/examples/hello_rtic.rs) and [hello_rtic_expanded.rs](rp2040-rtic/examples/hello_rtic_expanded.rs) which shows what the application expands to



