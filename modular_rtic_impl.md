### Modular RTIC Implementation details 

To achieve modularity and separation of concerns, `rtic-core` and other `standard compilation pass crates` expose a set of interfaces for the **distribution crate** to provide hardware specific details to be included during the code generation.



#### `rtic-core` interfaces:

##### Single core hardware pass trait (ScHwPassImpl) 

This trait acts as an interface between the `distribution crate`  and  `rtic-core` to provide hardware/architecture specific details needed during the code generation of the *hardware tasks and resources pass*.

```rust
/// Interface for providing hw/architecture specific details for implementing the hardware and resources pass
pub trait ScHwPassImpl {
    /// Code to be inserted after the call to init() and tasks init() functions
    /// This can for example enable interrupts used by the user and set their priorities
    fn post_init(
        &self,
        app_info: &ParsedRticApp,
        app_analysis: &AppAnalysis,
    ) -> Option<TokenStream2>;

    /// Fill the body of the rtic internal critical section function with hardware specific implementation.
    /// Use [eprintln()] to see the `fn_sign` requred function signature 
    fn fill_interrupt_free_fn(&self, fn_sign: syn::ItemFn) -> syn::ItemFn;

    /// Based on the information provided by the parsed application, such as Shared Resources priorities
    /// and Tasks priorities. Optionally return the generated code for statically stored priority masks
    fn compute_priority_masks(
        &self,
        app_info: &ParsedRticApp,
        app_analysis: &AppAnalysis,
    ) -> Option<TokenStream2>;

    /// Complete the implementation of the lock function for resource proxies
    /// Use [eprintln()] to see the `incomplete_lock_fn` singature and already provided logic inside it 
    fn impl_lock_mutex(&self, incomplete_lock_fn: syn::ItemFn) -> syn::ItemFn;

    /// Implementation for WFI (Wait for interrupt) instruction to be (optionally) used in default idle task
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
    let mc_pass =  /* Initialize multi-core struct from the Multi-core pass crate */;
    let other_pass1 =  /* Initialize pass from 3rd party crate */;
    let other_pass2 =  /* Initialize pass from 3rd party crate  */;
    
    let mut builder = RticAppBuilder::new(hw_pass); // Hardware pass is mandatory, other passes aren't
    builder.add_compilation_pass(CompilationPass::SwPass(sw_pass));
    builder.add_compilation_pass(CompilationPass::McPass(mc_pass));
    builder.add_compilation_pass(CompilationPass::MonotonicsPass(mono_pass));
    builder.add_compilation_pass(CompilationPass::Other(other_pass1));
    builder.add_compilation_pass(CompilationPass::Other(other_pass2));
    builder.build_rtic_application(args, input)
}
```



#### `rtic-sw-pass` interface

The `rtic-sw-pass` crate provides the `ScSoftwarePass` type which stands for Single core software pass and it already implements `RticPass` trait. An instance of type can be directly passed to `rtic-core` as a `CompilationPass::SwPass(...)` to add additional syntax for software tasks in you RTIC distibution .

The only thing that needs to provided by the ditribution create the is implementation of the `pend()` function. Which can be provided though the following interface:

```rust 
/// Interface for providing the hardware specific details needed by the single-core software pass
pub trait ScSoftwarePassImpl {
    /// Fill the body of the rtic internal pend() function with hardware specific implementation.
    /// Use [eprintln()] to see the `empty_body_fn` function signature
    fn fill_pend_fn(&self, empty_body_fn: syn::ItemFn) -> syn::ItemFn;
}
```

----

### Putting it all together: An rp2040 single core RTIC application implementation

To demonstrate how easy it is to create RTIC distributions, This example will show how an RP2040 single core rtic distribution can be built. 



First, let's start by providing the hadware specific details for the Hadware tasks and resources pass by implementing the `ScHwPassImpl` trait:

```rust
impl ScHwPassImpl for Rp2040HwPassBackend {
    fn post_init(
        &self,
        app_info: &rtic_core::ParsedRticApp,
        app_analysis: &rtic_core::AppAnalysis,
    ) -> Option<proc_macro2::TokenStream> {
        /* generate code for initializing interrupts and setting their priorities */
        /* for a real example see the implemnetation in rp2040-rtic crate in this git repo */
    }

    fn wfi(&self) -> Option<proc_macro2::TokenStream> {
        Some(quote! { unsafe { core::arch::asm!("wfi" ); } })
    }

    fn fill_interrupt_free_fn(&self, mut empty_body_fn: ItemFn) -> ItemFn {
        // use eprintln!("{}", empty_body_fn.to_token_stream().to_string()); 
        // enable above comment to see the required function signature
        let fn_body = parse_quote! {
            {
                unsafe { core::arch::asm!("cpsid i"); } // critical section begin
                let r = f();
                unsafe { core::arch::asm!("cpsie i"); } // critical section end
                r
            }
        };
        empty_body_fn.block = Box::new(fn_body);
        empty_body_fn
    }

    fn compute_priority_masks(
        &self,
        app_info: &ParsedRticApp,
        _app_analysis: &AppAnalysis,
    ) -> proc_macro2::TokenStream {
        /* for a real example see the implemnetation in rp2040-rtic crate in this git repo */
    }

    fn impl_lock_mutex(&self) -> proc_macro2::TokenStream {
        // deligate task to some function provided in exports
        // see rp2040-rtic/src/export.rs
        quote! {
            unsafe {rtic::export::lock(resource, task_priority, CEILING, &__rtic_internal_MASKS, f);}
        }
    }
}
```



Then lets do the same for software pass:

```rust
struct SwPassBackend;
impl ScSoftwarePassImpl for SwPassBackend {
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

### Using the resulting Single-core Rp2040 RTIC Framework

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
        
    #[task(priority = 1, shared = [shared2])]
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

**NOTE about changed syntax:** the way of **declaring a task now uses a struct that implements a specific task trait instead of a function** with Context,  the way of **defining local resources is by defining member variables of the Task struct**. and finally, shared resources are accessed through the `shared()` API, instead of using Context. The reason for this change is purely because this allows rapid prototyping due to the fact that it is much easier/faster to implement such model (and IMHO also feels less complicated/ more intuitive for first time users ).



For a real example on the rp2040 see:

-  [blinky.rs](rp2040-rtic/examples/blinky.rs) and [__expanded.rs](rp2040-rtic/examples/__expanded.rs) which shows what the application expands to



