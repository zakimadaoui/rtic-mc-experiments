#### Multi-core RTIC application declarative model

```rust

#[rp2040_rtic::app(
    device=rp2040_hal::pac,
    cores=2,
    dispatchers=[[/*dispatchers for core 1*/], [/*dispatchers for core 2*/], ..]
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
            if let Err(val, reason) =  Core2SwTask::spawn_from(self.current_core(), 7) {
                // do something on error case,
                // reason could be that task is spawned from incorrect core.
            }
            // self.current_core() returns a constant, zero-cost type for Core 1 that allows
            // static analysis to inforce the fact that only Core 1 can spawn this task on Core 2
         }
    }
    
    /// Software task to be spawned by core 1 on core 2  
    /// this task is automatically assinged to core 2 becaused it uses shared3 resource
    #[task(priority = 1, shared = [shared3], spawn_by = 1)]
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
- In addition, cross-core tasks of the same priority group must all have the same spawn_by index. I,e a dispatcher for cross core tasks can only serve one SPAWNER core.
- Priority of cross-core tasks cannot overlap with the priority of core-local tasks (they must have different dipatchers)

The above Constraints will be inforced at compile time and violations can be detected during the analysis phase.



-  and additional `spawned_by` attribute argument will be needed for software tasks to allow analysis to be performed to 
  1. ensure no race conditions occur when cores spawn tasks
  2. deciding which interrupt pending function to use. pend() or cross_pend() is generated inside the spawn API call impelemntation based on the `spawned_by = N`  argument when compared to `core` assigned to the task. if they are the same, then used pend() is used, otherwize use cross_pend().
  3. pend() and cross_pend() specific implementation will be both provided by the `RTIC distribution` alongside other hardware specific details. 
  4. When the argument is not provided it is automatically assumed that the task will be spawned locally. ie, spawn_by == core



### Open questions/problems about Multi-core RTIC and some answers

#### Solved problems so far

1. how to initialize the other core ? knowing that some architectures allow core A to start core B while others don't
   - solution: use post-init() API when called for core 1 to init core 2 (tested and already works)
2. Should there be multi-core pass ? or should the standard pass and software pass be re-factored to be generic enought to fit multicore applications. 
   1. from experiments, the second option seems to be the most viable. The main reasons are:
        - to have a multi-core application with multiple entries, the standard-pass must be able to detect that.
        - multi-core software tasks implementation are 98% similar to core-local task with the only exception beign the cross_pend() instead of pend()
        - it is easier to distribute tasks to dispatchers and make analysis and validation when core-local and cross-core software tasks

    2. in addition to re-factoring the standard-pass and software-pass to allow generic multi-core applications. An extra compilation pass or more can be added to perform automatic task assignment to cores, and application and memory partitioning

3. how to analyse multi-core application and have certain guarantees that spawning is done correctly (i,e no undefined behavior is allowed and no situtation that leads to race conditions)?
   1. -> already solved, see implementation of sw pass and standard pass (analysis part mostly)

4. How to have two or more Main functions ?
  - sol -> standard pass generates multiple entry functions (one entry for each core) and the distribution has some API to give names to the entries (on rp2040 rtic, we name the first entry `main` and the second entry some random name like `core1_entry`)
5. how and when to use `cross-pend()` and when to `pend()`
   - sol -> compare spawn_by vs core argument of software task
6. where and how to initialize mailboxes
   1. sol -> rtic-core already provides `post_init()` API which can be also used for initializing the mailboxes on both cores.


#### Yet to be solved 

1. How to partition code into different ram sections ? What convensions need to be followed on each compilation pass implementation to allow such thing ?
   - a solution could be to add  #[linker_section = ".text.coreX.context"] and context can be (globals, isr, task, ...). then use a custom linker script provided with the distribution. But all compilation passes have to conform to this practice or we get a broken application. In addition a linker script is needed for each distribution
2. How allow compiling a single source application to multiple binaries (for multic-core MCUs that require multi-binaries like the stm32h7 series) ?
