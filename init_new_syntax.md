### Potential new syntax for task initialization


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

        let (_uart_rx, uart_tx) = uart.split();
        #[init_task]
        MyHwTask::initialize(uart_tx); // initialize is auto generated

        // do something else

        #[init_task]
        MySwTask::initialize(7, false); // initialize is auto generated

        // any other (zero-size) struct, has a default implementation of initialize(), and if user doesn't call it explicitly
        // it gets initialized after global init() function 

        SharedResources { 
        	shared1 : val,
            shared2 : val,
        }
    }

    #[task(binds = TIMER_IRQ_0 , priority = 3, shared = [shared1, shared2])]
    struct MyHwTask {
        a1 : UartTx,
    }
    impl RticTask for MyHwTask {
        fn exec(&mut self) {
            /* ... */  
        }
    }

    #[sw_task(priority = 2, shared = [shared1])]
    struct MySwTask {
        /* local resources */
        b1 : u32,
        b2 : bool,
    }
    impl RticSwTask for MySwTask {
        type SpawnInput = u32;
        fn exec(&mut self, input: u32) {
            /* ... */ 
        }
    }
}
```