# Modular RTIC Implementation details 

To be able to decouple the hardware details from the declaratic model parsing and generation the `rtic-core (standard pass)` and other `compilation pass crates` expose a set of traits that can be implemented at the **distribution crate** level. The **distribution crate** aids the generic code generation part by suppling the missing hardware specific details though implementing those traits and dynamically passing the implementation.  

### Traits in `rtic-core`:

#### StandardPassImpl

This trait acts as an interface between the `distribution crate`  and  `rtic-core` to provide hardware/architecture specific details needed during the code generation of the *hardware tasks and resources pass*.

```rust
/// Interface for providing hw/architecture specific details for implementing the standard tasks and resources pass
pub trait StandardPassImpl {
    /// Return the default task priority to be used in idle task and tasks where priority argument is not mentioned
    fn default_task_priority(&self) -> u16;
    /// Code to be inserted after the call to Global init() and task init() functions
    /// This can for example enable interrupts used by the user and set their priorities.
    /// A SubApp and SubAnalysis correspond to a single-core appliation. 
    /// This function will be called several times depending on how many cores the user defines and the implementation supportes
    fn post_init(
        &self,
        app_args: &AppArgs,
        app_info: &SubApp,
        app_analysis: &SubAnalysis,
    ) -> Option<TokenStream2>;

    /// Implement the body of the rtic internal critical section function with hardware specific implementation (could be used as proxy for interrupt::free).
    /// Use [eprintln()] to see the `empty_body_fn` function signature
    fn impl_interrupt_free_fn(&self, empty_body_fn: syn::ItemFn) -> syn::ItemFn;

    /// Based on the information provided by the parsed sub-application, such as Shared Resources priorities
    /// and Tasks priorities. Return the generated code for statically stored priority masks.
    /// A SubApp and SubAnalysis correspond to a single-core appliation. 
    /// This function will be called several times depending on how many cores the user defines and the implementation supportes
    fn compute_priority_masks(
        &self,
        app_args: &AppArgs,
        app_info: &SubApp,
        app_analysis: &SubAnalysis,
    ) -> TokenStream2;

    /// Complete the implementation of the lock function for resource proxies
    /// Use [eprintln()] to see the `incomplete_lock_fn` singature and already provided logic inside it 
    /// see rp2040-rtic/rp2040-rtic-macro/src/lib.rs for more details
    fn impl_lock_mutex(&self, incomplete_lock_fn: syn::ItemFn) -> syn::ItemFn;

    /// Entry name for specific core
    /// This function is useful when there are multiple entries (multi-core app)
    /// and only one entry needs to be named `main`, but also the name of the other 
    /// entries needs to be known at the distribution crate level for other uses.
    fn entry_name(&self, core: u32) -> Ident;

    /// Implementation for WFI (Wait for interrupt) instruction to be used in default idle task
    fn wfi(&self) -> Option<TokenStream2>;
}
```


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



