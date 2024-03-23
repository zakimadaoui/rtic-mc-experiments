#### POTENTIAL Multi-core RTIC application declarative model

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



#### **Design decisions regarding multi-core extension:**

For the sake of sipmilification, let's assume that we have a dual core system (but the same decisions scale for N-core systems)

- If core A spawns a task on core B, the same task cannot be spawned from core B. Similarly if core A spawns a task locally, the same task cannot be spawned by another core. This decision is there to avoid a race condition across cores when inserting a ready task to some ready tasks priority queue during the call to `spawn()`. 
- Due to the previous condition, A Dispatcher will be **reserved** entirely for either 
  - tasks that are spawned locally (local core message passing)
  - tasks that are spawned by the other core (cross-core message passing in One Direction)
  - and it is **forbidden** to have a dispatcher that sevrves both the above purposes due to race conditions

The above Constraints will be inforced at compile time and violations will be detected during the analysis phase.



-  and additional `spawned_by` attribute argument will be needed for software tasks to allow analysis to be performed to 
  1. ensure no race conditions occur when cores spawn tasks
  2. deciding which interrupt pending function to use. pend() or cross_pend() is generated inside the spawn API call impelemntation based on the `spawned_by = N`  argument when compared to `core` assigned to the task. if they are the same, then used pend() is used, otherwize use cross_pend().
  3. pend() and cross_pend() specific implementation will be both provided by the `RTIC distribution` among other details. 
  4. When the argument is not provided it is automatically assumed that the task will be spawned locally



### Open questions about Multi-core pass and some answers

1. how to initialize the other core ? knowing that some architectures allow core A to start core B while others don't
   - a solution could be to use post-init of core 1 to init core 2
2. how to partition code into different RAM sections
   - a solution could be to add  #[linker_section = ".text.coreX.context"] and context can be (globals, isr, task, ...). then use a custom linker script provided with the distribution 
3. What should a multi-core pass do ? 
   - partition code into different ram sections ?
   - split single app to multiple app modules ?
   - how cross-pend pending is used and how does this affect software tasks pass ?
   - how the analysis to guarantee spawning is done correctly will be done ?
   - How to have two or more Main functions ?
     - hardware pass needs to know which main function each core uses since it needs to make calls to init, pre-init and idle code in
     - how to know the entry of the other core !?
   - how and when to use `cross-pend()` and when to `pend()`
4. is a multi-core pass is enough or do other passes (hadware and software pass especially) need to change behavior to expect multi-core code ?

