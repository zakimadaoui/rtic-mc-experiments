# Modular RTIC and POC for a multi-core extension on rp2040

**Experiment Objective:** 

The objective of this experiment is to enhance the scalability of the RTIC (Real-Time Interrupt-driven Concurrency) framework, particularly to allow for multicore hardware configurations. The goal is to decouple the RTIC declarative model from hardware-specific implementation details. And (possibly) provide a mechanism for additional syntax extension through 3rd party libraries. By achieving this, we aim to create a more maintainable and extensible RTIC framework capable of accommodating various hardware configurations while preserving the original core declarative model.



### Glossary

- `RTIC distribution`: is a crate that exposes the RTIC framework that is implemented for a specific hardware architecture or even for a specific microcontroller. For example, we could have a distribution for single core cortex-m devices, another distribution specifically tailored for the RP2040, a distribution for risc-v architecture ... etc. Each distribution will have the hardware specific details described in its own crate. This makes RTIC codebase growth more controllable and provides an alternative approach to the current one in which all the hardware specific details all belong to a single crate and an architecture is chosen by enabling a corresponding rust feature. 

  - RTIC distributions do not re-impelement the RTIC framework from scratch, instead they only provide the hardware specific parts of the implementation to `rtic-core` library  and other `compilation passes` crates/libraries that will do all the heavy lifting of parsing, analysing and generating code .

  - `rtic-core` and other `compilation passes` crates the RTIC  declarative model. Meaning that they all the parsing, syntax validation, and code generation. However, the code generation logic inside it is abstract and details related to specific hadware is provided by the `distribution` crate through some interfaces which we will mention later.
  - Note: `rtic-core` naming was chosen for lack of a better name. However, this lib is completely different from the other `rtic-core` crate in the original RTIC project. 

- `Compilation passes:` in short....
- one reason more why to pass implementation details to passes is that standard passes don't need to be re-written for different hw archs, instead it can be configured to directly support it.



## Declarative model for single-core, multi-core applications, and user plug-ins

before looking into the nasty implementation details, let's first get an overview how different RTIC applications should look like.



**NOTE1:** Because this is just an experiment, `rtic-core` implements only a subset of the Original RTIC declarative model. Currently only Init, idle, Hardware task, Software tasks with message passing, local and shared resources are provided ( monotonics, and async not implemented yet)

**NOTE about changed syntax:** the way of **declaring a task now uses a struct that implements a specific task trait instead of a function** with Context,  the way of **defining local resources is by defining member variables of the Task struct**. and finally, shared resources are accessed through the `shared()` API, instead of using Context. The reason for this change is purely because this allows rapid prototyping due to the fact that it is much easier/faster to implement such model (and IMHO also feels less complicated/ more intuitive for first time users ).



#### Single core application

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

in **Cargo.toml** of this application the feature `sw_tasks` must be enabled to allow the use of the `dispatchers = [...]` attribute and to allow declaring software tasks. Similarly, if the user wants to use some extended features of RTIC (such as monotonics or async) , the user can enable such *<u>internal-extensions</u>* through their appropriate features. Note however, that not all **rtic distributions** will support all the internal-extensions and it is up to the **distributor** to support and expose a sub-set or all features allowed by `rtic-core`. 

   

#### Multi-core RTIC application

```rust

#[rp2040_rtic::app(
    device=rp2040_hal::pac,
    cores=2,
    dispatchers=[[/*dispatchers for core 1*/], [/*dispatchers for core 2*/]]
    ),
]
pub mod my_multicore_core_app {

    // user includes ...
    
    #[shared(core=1)]
    struct SharedResources1 {
        shared1 : Type,
        shared2 : Type,
    }
    
    #[shared(core=2)]
    struct SharedResources2 {
        shared3 : Type,
        shared4 : Type,
    }

    #[init(core=1)]
    fn init1() -> SharedResources1 {
        // init code ....
        SharedResources1 { 
        	shared1 : val,
            shared2 : val,
        }
    }
    
    #[init(core=2)]
    fn init2() -> SharedResources2 {
        // init code ....
        SharedResources2 { 
        	shared3 : val,
            shared4 : val,
        }
    }

    /// Hardware task with automatic core assignment
    /// this task will be assigned to core1 because it uses shared1 resource
    #[task(binds = TIMER_IRQ_0 , priority = 3, shared = [shared1], core = auto /*by default*/)]
    struct MyHwTask1 {
        /* local resources */
    }
    impl RticTask for MyHwTask1 {
        fn init() -> Self { Self { /* init local resources */ } }
        fn exec(&mut self) { /* task code ... */ }
    }
        
    
    /// Hardware task with static core assignment that Spawns a task on the other core
    #[task(binds = TIMER_IRQ_1, priority = 1, core = 1]
    struct MyHwTask2 {
        /* local resources */
    }
     impl RticTask for MyHwTask2 {
        fn init() -> Self { Self { /* init local resources */ } }
        fn exec(&mut self) { 
            /* task code ... */ 
            if let Err(val, reason) =  Core2SwTask::spawn(7, CURRENT_CORE) {
                // do something on error case,
                // reason could be that task is spawned from incorrect core.
            }
            // NOTE: CURRENT_CORE is an auto generated constant used to validate that the correct 
            // core is used to spawn the task.
         }
    }
    
    /// Software task assigned to core 2 (becaused it uses shared3 resource) to be spawned by core 1
    #[task(priority = 1, shared = [shared3], spawned_by = 1)]
    struct Core2SwTask {
        /* local resources */
    }
    impl RticSwTask for Core2SwTask {
        type TaskInputType = u8;
        
        fn init() -> Self {
            Self { /* init local resources */ }
        }

        fn exec(&mut self, input: u8) {
            // do something with input
            // lock `shared3` resource 
            // use your creativity
        }
    }

    
}
```



**Design decisions regarding multi-core extension:**

For the sake of sipmilification, let's assume that we have a dual core system (but the same decisions scale for N-core systems)

1. A software task can be spawned exclusively by one of the two cores. Meaning that you can't have tasks from both core1 and core2  spawn the same software task as that would result in a race condition when accessing the priority queue. More over, a priority line for software tasks will be reserved exclusively from tasks spawned from one of the cores (for the same reason as earlier). 
   - These constraints are inforced at compile time and will be detected during the analysis phase.
2.  Due to the earlier condition, and additional `spawned_by` argument will be needed for software tasks to allow analysis to be performed to 
   1. ensure exclusivity of the core spawning the tasks
   2. deciding which interrupt pending function to use. pend() or cross_pend() is generated inside the spawn API call impelemntation based on the `spawned_by = N`  argument when compared to `core` assigned to the task. if they are the same, then used pend() is used, otherwize use cross_pend().
   3. pend() and cross_pend() specific implementation will be both provided by the `RTIC distribution` among other details. 



### Implementation details

#### rtic-core and rtic distributions 

To achieve modularity and separation of concerns, `rtic-core` library has been designed in a way such that it:

1. **Provides Parsing and Analysis Logic:** The `rtic-core` library contains all the  logic required for parsing and analyzing an RTIC application. 

2. **Abstract Code Generation:** `rtic-core` abstracts most of the code generation process, ensuring that it remains hardware-agnostic. However, when hardware-specific code is necessary, the library facilitates the integration of such details using a **Plug-in mechanism** which any arbitrary external crate known as the `implementor crate` can provide the hardware specific implementation details.

3. **Plug-in Mechanism:** The integration of hardware-specific details is facilitated through a plug-in mechanism. The is mechanism works by exposing a set of traits, Each trait defined by `rtic-core` represents a distinct aspect of the RTIC framework, such as core functionality (like resource locking), and other functionality like  monotonics ..etc.

   These traits define a contract for how the implementor should provide the hardware-specific details.

   The `implementor crate`, which serves as the actual RTIC proc macro used by the user, leverages these traits to provide hardware-specific implementations. In addition, this approach allows extending the RTIC declarative model beyond what `rtic-core` provides, such as adding multi-core support for some specific target.

![rtic-core-modular.png](rtic-core-modular.png)



#### Example of Implementing a Single Core RTIC distribution for rp2040

```rust
use proc_macro::TokenStream;
use quote::quote;
use rtic_core::{ParsedRticApp, RticCoreImplementor, AppAnalysis};
extern crate proc_macro;

// single-core hadware tasks and resources implementation (used when generating hw tasks and resources)
struct Rp2040Hw;
impl ScHwImplementation for Rp2040Hw {
    fn get_default_task_prio(&self) -> u16 {0}
    fn get_min_task_prio(&self) -> u16 {1}
    fn get_max_task_prio(&self) -> u16 {3}

    fn pre_init(&self, app_info: &ParsedRticApp, app_analysis: &AppAnalysis) -> Option<proc_macro2::TokenStream> {
        // implementation of pre-initialization code, this can for example include enabling interrupts
        // and setting their priorities. All the information needed about the used interrupts used
        // in the application and their associated priorities can be found in `app_info`.
       	None
    	}

    fn critical_section_begin(&self) -> proc_macro2::TokenStream {
        quote! {unsafe { core::arch::asm!("cpsid i"); }}
    }

    fn critical_section_end(&self) -> proc_macro2::TokenStream {
		quote! {unsafe { core::arch::asm!("cpsie i" ); }}
    }

    fn wfi(&self) -> Option<proc_macro2::TokenStream> {
        Some(quote! {unsafe { core::arch::asm!("wfi" ); }})
    }

    fn compute_priority_masks(&self, app_info: &ParsedRticApp, app_analysis: &AppAnalysis) -> proc_macro2::TokenStream {
        /* see how this is implemented in rp2040-rtic */
    }

    fn impl_lock_mutex(&self) -> proc_macro2::TokenStream {
        /* see doc comment in `rtic-core` for this function for more details */
        /* lock() function is defined in `rp2040-rtic/src/export.rs` */
        quote!{
            unsafe {rtic::export::lock(resource, task_priority, CEILING, &__rtic_internal_MASKS, f);}
        }
    }
}

// for implementing software tasks we need to provide how to pend and interrupt
struct Rp2040Sw;
impl ScSwImplementation for Rp2040Sw {
    fn pend_irq(irq_name: syn::Ident) -> proc_macro2::TokenStream {
        /* NVIC is re-exported in `rp2040-rtic/src/export.rs` */
        quote!{ rtic::export::NVIC::pend(Interrupt::#irq_name); }
    }
    // other trait methods ...
}

use rtic_core::RticAppBuilder;

#[proc_macro_attribute]
pub fn rp2040_rtic(args: TokenStream, input: TokenStream) -> TokenStream {
    // pass the implementation to RticAppBuilder and call parse(), its that simple !
    let mut builder = RticAppBuilder::new();
    builder.hw_impl(HwImpl::SingleCore(Rp2040Hw)); // the only mandatory implementation to be passed
    builder.sw_impl(SwImpl::SingleCore(Rp2040Sw)); // sw tasks implementation (optional, to enable sw tasks)
    // builder.monotonics_impl(Rp2040RticMonotonics); // monotonics implementaion (also optional ..)
    // builder... // other modular aspects of RTIC that can be introduced in future
    if let Err(e) = builder.parse(args, input) {
		// handle error, 
    }
    
}
```



#### Example of Implementing a Multi Core RTIC distribution for rp2040





For a real example see `rp2040-rtic`. (currently still work in progress.)



### The declarative model `rtic-core` provides:

-  [blinky.rs](rp2040-rtic/examples/blinky.rs) is a very simple example that showcases  the `rtic-core` declarative model for a single core application
- [blinky_expanded.rs](rp2040-rtic/examples/blinky_expanded.rs) shows what the application expands to





#### How can `rtic-core` library facilitate the implementation of multi-core ?

There are two problems that we need to solve if we need to make the multi-core extension for the RP2040:

##### 1. Core1 has to initialize Core2, and how can this be hidden from the user:

This problem can be solved by implementing `RticCoreImplementor` **twice**;

-  where in Core1 implementation of `RticCoreImplementor` trait, the  `pre_init()` function will include **initializing core2 and waking it up** in addition to other common pre-initialization code like enabling local interrupts and setting up their priorities.  
- So, when the `rp2040_rtic` proc-macro function gets called the first time to parse the first RTIC application and it sees the attribute `core = 1`, then It passes Core1 implementation of `RticCoreImplementor` to `RticAppBuilder`. 



##### 2. Message passing, local vs multi-core:

single core message passing support in `rtic-core` will soon be added, this will include:

- generating a **ready-queue** for each priority level 
- generating a **inputs-queue** for each task
- generating a **finished-queue** for each task
- binding the software task to the appropriate dispatcher
- However, the actual implementation of the `pend(irq)` function using in triggering the dispatcher will be requested from the **Implementor** though the   `RticCoreImplementor` trait to provide the hardware specific details. 



The multi-core extension, however, will present the following two challenges (especially the second):

1. a different `pend()` function needs to be used when doing cross-core message passing. and as a solution, the `rtic-core` library, can facilitate this by exposing an optional function in `RticCoreImplementor` called `cross_pend()` which abstracts the implementation  of or cross-node interrupt pending.
2. the second challenge is how the access to the **read, inputs and finished** queues can be governed in the case of two or more cores and how can this be abstracted away
   1. one approach could be to protect these **queues**  using hardware spinlocks  
   2. another approach, would be to have separate queues (one for core-local message passing and another for cross-core message passing) but this will also introduce some overhead in dispatching tasks and will need to be generic enough to be abstracted away in `rtic-core`

The solution to the second challenge is yet to be determined...







### Arguments why the user should not have full control over the passes

1. it adds extra complexity for a first time user. (now they have to think about compilation passes !!)
2. the user must always keep the passes in the right order
3. some 3rd party compilation passes can cause interference with other passes and cause undefined behavior making debugging a hell.
4. in best case scenario application wouldn't compile but will give strange errors because some pass is missing some  auto generated code from another pass which is not the responsibility of the user to add



The alternative approach is to allow distributors to pick the most stable compilation passes for the specific target and guard-them behind features. This will automatically guarantee that the compilation passes are called in the right order, in addition the compilation passes will also be guaranteed to work as intended with no undefined behavior. 

