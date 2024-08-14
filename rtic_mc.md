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

