pub mod my_app
{
    #[doc = r" Include peripheral crate that defines the vector table"] use
    rp2040_hal :: pac as _ ;
    #[doc =
    r" ================================== user includes ===================================="]
    use cortex_m :: asm ; use defmt :: * ; use defmt_rtt as _ ; use
    panic_probe as _ ; use rp2040_hal :: fugit :: MicrosDurationU32 ; use
    rp2040_hal :: gpio :: bank0 :: Gpio25 ; use rp2040_hal :: gpio ::
    { FunctionSio, Pin, PullDown, SioOutput } ; use rp2040_hal :: timer ::
    { Alarm, Alarm0 } ; use embedded_hal :: digital :: v2 :: OutputPin ; use
    rp2040_hal :: pac :: { self, Interrupt } ;
    #[doc =
    r" ==================================== rtic traits ===================================="]
    pub use rtic_traits :: * ; #[doc = r" Module defining rtic traits"] mod
    rtic_traits
    {
        #[doc = r" Trait for a hardware task"] pub trait RticTask
        {
            #[doc = r" Task local variables initialization routine"] fn init()
            -> Self ; #[doc = r" Function to be bound to a HW Interrupt"] fn
            exec(& mut self) ;
        } #[doc = r" Trait for an idle task"] pub trait RticIdleTask
        {
            #[doc = r" Task local variables initialization routine"] fn init()
            -> Self ;
            #[doc =
            r" Function to be executing when no other task is running"] fn
            exec(& mut self) ->! ;
        } pub trait RticMutex
        {
            type ResourceType ; fn
            lock(& mut self, f : impl FnOnce(& mut Self :: ResourceType)) ;
        }
    }
    #[doc =
    r" ================================== rtic functions ==================================="]
    #[doc = r" critical section function"] #[inline] pub fn
    __rtic_interrupt_free < F, R > (f : F) -> R where F : FnOnce() -> R,
    {
        unsafe { core :: arch :: asm! ("cpsid i") ; } let r = f() ; unsafe
        { core :: arch :: asm! ("cpsie i") ; } r
    }
    #[doc =
    r" ==================================== User code ======================================"]
    type LedOutPin = Pin < Gpio25, FunctionSio < SioOutput >, PullDown > ;
    static DELAY : u32 = 1000 ; static mut __rtic_internal__MyTask2__INPUTS :
    rtic :: export :: Queue < < MyTask2 as RticSwTask > :: SpawnInput, 2 > =
    rtic :: export :: Queue :: new() ; impl MyTask2
    {
        pub fn spawn(input : < MyTask2 as RticSwTask > :: SpawnInput) ->
        Result < (), < MyTask2 as RticSwTask > :: SpawnInput >
        {
            let mut inputs_producer = unsafe
            { __rtic_internal__MyTask2__INPUTS.split().0 } ; let mut
            ready_producer = unsafe
            { __rtic_internal__Core0Prio2Tasks__RQ.split().0 } ;
            #[doc =
            r" need to protect by a critical section due to many producers of different priorities can spawn/enqueue this task"]
            __rtic_interrupt_free(| | -> Result < (), < MyTask2 as RticSwTask
            > :: SpawnInput >
            {
                inputs_producer.enqueue(input) ? ; unsafe
                {
                    ready_producer.enqueue_unchecked(Core0Prio2Tasks :: MyTask2)
                } ;
                __rtic_sc_pend(rp2040_hal :: pac :: Interrupt :: DMA_IRQ_0 as
                u16) ; Ok(())
            })
        }
    } static mut __rtic_internal__MyTask7__INPUTS : rtic :: export :: Queue <
    < MyTask7 as RticSwTask > :: SpawnInput, 2 > = rtic :: export :: Queue ::
    new() ; impl MyTask7
    {
        pub fn spawn(input : < MyTask7 as RticSwTask > :: SpawnInput) ->
        Result < (), < MyTask7 as RticSwTask > :: SpawnInput >
        {
            let mut inputs_producer = unsafe
            { __rtic_internal__MyTask7__INPUTS.split().0 } ; let mut
            ready_producer = unsafe
            { __rtic_internal__Core0Prio2Tasks__RQ.split().0 } ;
            #[doc =
            r" need to protect by a critical section due to many producers of different priorities can spawn/enqueue this task"]
            __rtic_interrupt_free(| | -> Result < (), < MyTask7 as RticSwTask
            > :: SpawnInput >
            {
                inputs_producer.enqueue(input) ? ; unsafe
                {
                    ready_producer.enqueue_unchecked(Core0Prio2Tasks :: MyTask7)
                } ;
                __rtic_sc_pend(rp2040_hal :: pac :: Interrupt :: DMA_IRQ_0 as
                u16) ; Ok(())
            })
        }
    } #[doc = " Dispatchers of"] #[doc = " Core 0"] #[derive(Clone, Copy)]
    #[doc(hidden)] pub enum Core0Prio2Tasks { MyTask2, MyTask7, }
    #[doc(hidden)] #[allow(non_upper_case_globals)] static mut
    __rtic_internal__Core0Prio2Tasks__RQ : rtic :: export :: Queue <
    Core0Prio2Tasks, 3usize > = rtic :: export :: Queue :: new() ; static mut
    __rtic_internal__MyCore1SwTask__INPUTS : rtic :: export :: Queue < <
    MyCore1SwTask as RticSwTask > :: SpawnInput, 2 > = rtic :: export :: Queue
    :: new() ; impl MyCore1SwTask
    {
        pub fn spawn(input : < MyCore1SwTask as RticSwTask > :: SpawnInput) ->
        Result < (), < MyCore1SwTask as RticSwTask > :: SpawnInput >
        {
            let mut inputs_producer = unsafe
            { __rtic_internal__MyCore1SwTask__INPUTS.split().0 } ; let mut
            ready_producer = unsafe
            { __rtic_internal__Core1Prio2Tasks__RQ.split().0 } ;
            #[doc =
            r" need to protect by a critical section due to many producers of different priorities can spawn/enqueue this task"]
            __rtic_interrupt_free(| | -> Result < (), < MyCore1SwTask as
            RticSwTask > :: SpawnInput >
            {
                inputs_producer.enqueue(input) ? ; unsafe
                {
                    ready_producer.enqueue_unchecked(Core1Prio2Tasks ::
                    MyCore1SwTask)
                } ;
                __rtic_sc_pend(rp2040_hal :: pac :: Interrupt :: I2C1_IRQ as
                u16) ; Ok(())
            })
        }
    } static mut __rtic_internal__CrossCoreTask__INPUTS : rtic :: export ::
    Queue < < CrossCoreTask as RticSwTask > :: SpawnInput, 2 > = rtic ::
    export :: Queue :: new() ; impl CrossCoreTask
    {
        pub fn
        spawn_from(_spawner : __rtic__internal__Core0, input : < CrossCoreTask
        as RticSwTask > :: SpawnInput) -> Result < (), < CrossCoreTask as
        RticSwTask > :: SpawnInput >
        {
            let mut inputs_producer = unsafe
            { __rtic_internal__CrossCoreTask__INPUTS.split().0 } ; let mut
            ready_producer = unsafe
            { __rtic_internal__Core1Prio1Tasks__RQ.split().0 } ;
            #[doc =
            r" need to protect by a critical section due to many producers of different priorities can spawn/enqueue this task"]
            __rtic_interrupt_free(| | -> Result < (), < CrossCoreTask as
            RticSwTask > :: SpawnInput >
            {
                inputs_producer.enqueue(input) ? ; unsafe
                {
                    ready_producer.enqueue_unchecked(Core1Prio1Tasks ::
                    CrossCoreTask)
                } ;
                __rtic_mc_pend(rp2040_hal :: pac :: Interrupt :: DMA_IRQ_1 as
                u16, 1u32) ; Ok(())
            })
        }
    } #[doc = " Dispatchers of"] #[doc = " Core 1"] #[derive(Clone, Copy)]
    #[doc(hidden)] pub enum Core1Prio2Tasks { MyCore1SwTask, } #[doc(hidden)]
    #[allow(non_upper_case_globals)] static mut
    __rtic_internal__Core1Prio2Tasks__RQ : rtic :: export :: Queue <
    Core1Prio2Tasks, 2usize > = rtic :: export :: Queue :: new() ;
    #[derive(Clone, Copy)] #[doc(hidden)] pub enum Core1Prio1Tasks
    { CrossCoreTask, } #[doc(hidden)] #[allow(non_upper_case_globals)] static
    mut __rtic_internal__Core1Prio1Tasks__RQ : rtic :: export :: Queue <
    Core1Prio1Tasks, 2usize > = rtic :: export :: Queue :: new() ;
    #[doc = r" RTIC Software task trait"] #[doc = r" Trait for an idle task"]
    pub trait RticSwTask
    {
        type SpawnInput ;
        #[doc = r" Task local variables initialization routine"] fn init() ->
        Self ;
        #[doc =
        r" Function to be executing when the scheduled software task is dispatched"]
        fn exec(& mut self, input : Self :: SpawnInput) ;
    } #[doc = r" Core local interrupt pending"] #[doc(hidden)] #[inline] pub
    fn __rtic_sc_pend(irq_nbr : u16)
    {
        unsafe
        {
            (* rtic :: export :: NVIC :: PTR).ispr
            [usize :: from(irq_nbr / 32)].write(1 << (irq_nbr % 32))
        }
    } #[doc(hidden)] #[inline] pub fn
    __rtic_mc_pend(irq_nbr : u16, core : u32)
    { rtic :: export :: cross_core :: pend_irq(irq_nbr) ; }
    #[doc = " ===================================="] #[doc = " CORE 0"]
    #[doc = " ==================================== "] static mut
    SHARED_RESOURCES : core :: mem :: MaybeUninit < SharedResources > = core
    :: mem :: MaybeUninit :: uninit() ; struct SharedResources
    { alarm : Alarm0, led : LedOutPin, } fn init_core0() -> SharedResources
    {
        let mut device = pac :: Peripherals :: take().unwrap() ; let cpu_id =
        device.SIO.cpuid.read().bits() ; info! ("staring core {} ...", cpu_id)
        ; let mut watchdog = rp2040_hal :: watchdog :: Watchdog ::
        new(device.WATCHDOG) ; let clocks = rp2040_hal :: clocks ::
        init_clocks_and_plls(12_000_000u32, device.XOSC, device.CLOCKS,
        device.PLL_SYS, device.PLL_USB, & mut device.RESETS, & mut
        watchdog,).ok().unwrap() ; let sio = rp2040_hal :: Sio ::
        new(device.SIO) ; let pins = rp2040_hal :: gpio :: Pins ::
        new(device.IO_BANK0, device.PADS_BANK0, sio.gpio_bank0, & mut
        device.RESETS,) ; let led_pin = pins.gpio25.into_push_pull_output() ;
        let mut timer = rp2040_hal :: Timer ::
        new(device.TIMER, & mut device.RESETS, & clocks) ; let mut alarm0 =
        timer.alarm_0().unwrap() ;
        alarm0.schedule(MicrosDurationU32 :: millis(DELAY)).unwrap() ;
        alarm0.enable_interrupt() ; SharedResources
        { alarm : alarm0, led : led_pin, }
    } static mut MY_IDLE_TASK : core :: mem :: MaybeUninit < MyIdleTask > =
    core :: mem :: MaybeUninit :: uninit() ; struct MyIdleTask
    { count : u32, } impl RticIdleTask for MyIdleTask
    {
        fn init() -> Self { Self { count : 0 } } fn exec(& mut self) ->!
        { loop { self.count += 1 ; asm :: delay(120000000) ; } }
    } impl MyIdleTask { pub const fn priority() -> u16 { 3u16 } } impl
    MyIdleTask
    {
        const fn current_core() -> __rtic__internal__Core0
        { unsafe { __rtic__internal__Core0 :: new() } }
    } static mut MY_TASK : core :: mem :: MaybeUninit < MyTask > = core :: mem
    :: MaybeUninit :: uninit() ; struct MyTask
    { is_high : bool, counter : u16, } impl RticTask for MyTask
    {
        fn init() -> Self { Self { is_high : false, counter : 0, } } fn
        exec(& mut self)
        {
            self.shared().led.lock(| led_pin |
            {
                if self.is_high
                { let _ = led_pin.set_low() ; self.is_high = false ; } else
                { let _ = led_pin.set_high() ; self.is_high = true ; }
            }) ; self.counter += 1 ; let message = self.counter ; if let
            Err(_e) = MyTask2 :: spawn(message)
            { error! ("couldn't spawn task 2 for the first time ") } if let
            Err(_e) = MyTask2 :: spawn(message)
            { error! ("couldn't spawn task 2 again") }
            self.shared().alarm.lock(| alarm0 |
            {
                let _ = alarm0.schedule(MicrosDurationU32 :: millis(DELAY)) ;
                alarm0.clear_interrupt() ;
            }) ;
        }
    } impl MyTask { pub const fn priority() -> u16 { 3u16 } } impl MyTask
    {
        pub fn shared(& self) -> __my_task_shared_resources
        {
            const TASK_PRIORITY : u16 = 3u16 ; __my_task_shared_resources ::
            new(TASK_PRIORITY)
        }
    } struct __my_task_shared_resources
    { pub alarm : __alarm_mutex, pub led : __led_mutex, } impl
    __my_task_shared_resources
    {
        #[inline(always)] pub fn new(priority : u16) -> Self
        {
            Self
            {
                alarm : __alarm_mutex :: new(priority), led : __led_mutex ::
                new(priority),
            }
        }
    } impl MyTask
    {
        const fn current_core() -> __rtic__internal__Core0
        { unsafe { __rtic__internal__Core0 :: new() } }
    } static mut MY_TASK3 : core :: mem :: MaybeUninit < MyTask3 > = core ::
    mem :: MaybeUninit :: uninit() ; struct MyTask3 ; impl RticTask for
    MyTask3 { fn init() -> Self { Self } fn exec(& mut self) {} } impl MyTask3
    { pub const fn priority() -> u16 { 1u16 } } impl MyTask3
    {
        pub fn shared(& self) -> __my_task3_shared_resources
        {
            const TASK_PRIORITY : u16 = 1u16 ; __my_task3_shared_resources ::
            new(TASK_PRIORITY)
        }
    } struct __my_task3_shared_resources { pub alarm : __alarm_mutex, } impl
    __my_task3_shared_resources
    {
        #[inline(always)] pub fn new(priority : u16) -> Self
        { Self { alarm : __alarm_mutex :: new(priority), } }
    } impl MyTask3
    {
        const fn current_core() -> __rtic__internal__Core0
        { unsafe { __rtic__internal__Core0 :: new() } }
    } static mut MY_TASK2 : core :: mem :: MaybeUninit < MyTask2 > = core ::
    mem :: MaybeUninit :: uninit() ; #[doc = " Software tasks of"]
    #[doc = " Core 0"] struct MyTask2 ; impl RticSwTask for MyTask2
    {
        type SpawnInput = u16 ; fn init() -> Self { Self } fn
        exec(& mut self, input : u16)
        {
            info! ("task2 spawned with input {}", input) ; if let Err(_e) =
            MyTask7 :: spawn(input + 10) { error! ("couldn't spawn task 7") }
            let _allowed_spawn = CrossCoreTask ::
            spawn_from(Self :: current_core(), 1) ;
        }
    } impl MyTask2 { pub const fn priority() -> u16 { 2u16 } } impl MyTask2
    {
        pub fn shared(& self) -> __my_task2_shared_resources
        {
            const TASK_PRIORITY : u16 = 2u16 ; __my_task2_shared_resources ::
            new(TASK_PRIORITY)
        }
    } struct __my_task2_shared_resources { pub led : __led_mutex, } impl
    __my_task2_shared_resources
    {
        #[inline(always)] pub fn new(priority : u16) -> Self
        { Self { led : __led_mutex :: new(priority), } }
    } impl MyTask2
    {
        const fn current_core() -> __rtic__internal__Core0
        { unsafe { __rtic__internal__Core0 :: new() } }
    } static mut MY_TASK7 : core :: mem :: MaybeUninit < MyTask7 > = core ::
    mem :: MaybeUninit :: uninit() ; struct MyTask7 ; impl RticSwTask for
    MyTask7
    {
        type SpawnInput = u16 ; fn init() -> Self { Self } fn
        exec(& mut self, input : u16)
        { info! ("task7 spawned with input {}", input) ; }
    } impl MyTask7 { pub const fn priority() -> u16 { 2u16 } } impl MyTask7
    {
        pub fn shared(& self) -> __my_task7_shared_resources
        {
            const TASK_PRIORITY : u16 = 2u16 ; __my_task7_shared_resources ::
            new(TASK_PRIORITY)
        }
    } struct __my_task7_shared_resources { pub led : __led_mutex, } impl
    __my_task7_shared_resources
    {
        #[inline(always)] pub fn new(priority : u16) -> Self
        { Self { led : __led_mutex :: new(priority), } }
    } impl MyTask7
    {
        const fn current_core() -> __rtic__internal__Core0
        { unsafe { __rtic__internal__Core0 :: new() } }
    } static mut CORE0_PRIORITY2_DISPATCHER : core :: mem :: MaybeUninit <
    Core0Priority2Dispatcher > = core :: mem :: MaybeUninit :: uninit() ;
    #[doc(hidden)] pub struct Core0Priority2Dispatcher ; impl RticTask for
    Core0Priority2Dispatcher
    {
        fn init() -> Self { Self } fn exec(& mut self)
        {
            unsafe
            {
                let mut ready_consumer =
                __rtic_internal__Core0Prio2Tasks__RQ.split().1 ; while let
                Some(task) = ready_consumer.dequeue()
                {
                    match task
                    {
                        Core0Prio2Tasks :: MyTask2 =>
                        {
                            let mut input_consumer =
                            __rtic_internal__MyTask2__INPUTS.split().1 ; let input =
                            input_consumer.dequeue_unchecked() ;
                            MY_TASK2.assume_init_mut().exec(input) ;
                        } Core0Prio2Tasks :: MyTask7 =>
                        {
                            let mut input_consumer =
                            __rtic_internal__MyTask7__INPUTS.split().1 ; let input =
                            input_consumer.dequeue_unchecked() ;
                            MY_TASK7.assume_init_mut().exec(input) ;
                        }
                    }
                }
            }
        }
    } impl Core0Priority2Dispatcher
    { pub const fn priority() -> u16 { 2u16 } } impl Core0Priority2Dispatcher
    {
        const fn current_core() -> __rtic__internal__Core0
        { unsafe { __rtic__internal__Core0 :: new() } }
    } #[allow(non_snake_case)] #[no_mangle] fn TIMER_IRQ_0()
    { unsafe { MY_TASK.assume_init_mut().exec() ; } } #[allow(non_snake_case)]
    #[no_mangle] fn TIMER_IRQ_2()
    { unsafe { MY_TASK3.assume_init_mut().exec() ; } }
    #[allow(non_snake_case)] #[no_mangle] fn DMA_IRQ_0()
    { unsafe { CORE0_PRIORITY2_DISPATCHER.assume_init_mut().exec() ; } }
    struct __alarm_mutex { #[doc(hidden)] priority : u16, } impl __alarm_mutex
    {
        #[inline(always)] pub fn new(priority : u16) -> Self
        { Self { priority } }
    } impl RticMutex for __alarm_mutex
    {
        type ResourceType = Alarm0 ; fn
        lock(& mut self, f : impl FnOnce(& mut Alarm0))
        {
            const CEILING : u16 = 3u16 ; let task_priority = self.priority ;
            let resource = unsafe
            { & mut SHARED_RESOURCES.assume_init_mut().alarm } as * mut _ ;
            unsafe
            {
                rtic :: export ::
                lock(resource, task_priority, CEILING, &
                __rtic_internal_MASKS_core0, f) ;
            }
        }
    } struct __led_mutex { #[doc(hidden)] priority : u16, } impl __led_mutex
    {
        #[inline(always)] pub fn new(priority : u16) -> Self
        { Self { priority } }
    } impl RticMutex for __led_mutex
    {
        type ResourceType = LedOutPin ; fn
        lock(& mut self, f : impl FnOnce(& mut LedOutPin))
        {
            const CEILING : u16 = 3u16 ; let task_priority = self.priority ;
            let resource = unsafe
            { & mut SHARED_RESOURCES.assume_init_mut().led } as * mut _ ;
            unsafe
            {
                rtic :: export ::
                lock(resource, task_priority, CEILING, &
                __rtic_internal_MASKS_core0, f) ;
            }
        }
    } #[doc = "Unique type for core 0"] pub use core0_type_mod ::
    __rtic__internal__Core0 ; mod core0_type_mod
    {
        struct __rtic__internal__Core0Inner ; pub struct
        __rtic__internal__Core0(__rtic__internal__Core0Inner) ; impl
        __rtic__internal__Core0
        {
            pub const unsafe fn new() -> Self
            { __rtic__internal__Core0(__rtic__internal__Core0Inner) }
        }
    } #[doc(hidden)] #[allow(non_upper_case_globals)] const
    __rtic_internal_MASK_CHUNKS_core0 : usize = rtic :: export ::
    compute_mask_chunks([rp2040_hal :: pac :: Interrupt :: TIMER_IRQ_0 as u32,
    rp2040_hal :: pac :: Interrupt :: TIMER_IRQ_2 as u32, rp2040_hal :: pac ::
    Interrupt :: DMA_IRQ_0 as u32,]) ; #[doc(hidden)]
    #[allow(non_upper_case_globals)] const __rtic_internal_MASKS_core0 :
    [rtic :: export :: Mask < __rtic_internal_MASK_CHUNKS_core0 > ; 3] =
    [rtic :: export ::
    create_mask([rp2040_hal :: pac :: Interrupt :: TIMER_IRQ_2 as u32,]), rtic
    :: export ::
    create_mask([rp2040_hal :: pac :: Interrupt :: DMA_IRQ_0 as u32,]), rtic
    :: export ::
    create_mask([rp2040_hal :: pac :: Interrupt :: TIMER_IRQ_0 as u32,]),] ;
    #[doc = r" Entry of "] #[doc = " CORE 0"] #[no_mangle] pub fn main() ->!
    {
        __rtic_interrupt_free(||
        {
            unsafe
            {
                MY_TASK.write(MyTask :: init()) ;
                MY_TASK3.write(MyTask3 :: init()) ;
                MY_TASK2.write(MyTask2 :: init()) ;
                MY_TASK7.write(MyTask7 :: init()) ;
                CORE0_PRIORITY2_DISPATCHER.write(Core0Priority2Dispatcher ::
                init()) ;
            } let shared_resources = init_core0() ; unsafe
            { SHARED_RESOURCES.write(shared_resources) ; } unsafe
            {
                rp2040_hal :: pac :: CorePeripherals ::
                steal().NVIC.set_priority(rp2040_hal :: pac :: Interrupt ::
                TIMER_IRQ_0, 3u16 as u8) ; rp2040_hal :: pac :: NVIC ::
                unmask(rp2040_hal :: pac :: Interrupt :: TIMER_IRQ_0) ;
                rp2040_hal :: pac :: CorePeripherals ::
                steal().NVIC.set_priority(rp2040_hal :: pac :: Interrupt ::
                TIMER_IRQ_2, 1u16 as u8) ; rp2040_hal :: pac :: NVIC ::
                unmask(rp2040_hal :: pac :: Interrupt :: TIMER_IRQ_2) ;
                rp2040_hal :: pac :: CorePeripherals ::
                steal().NVIC.set_priority(rp2040_hal :: pac :: Interrupt ::
                DMA_IRQ_0, 2u16 as u8) ; rp2040_hal :: pac :: NVIC ::
                unmask(rp2040_hal :: pac :: Interrupt :: DMA_IRQ_0) ;
            } #[doc = r" Stack for core 1"] #[doc = r""]
            #[doc =
            r" Core 0 gets its stack via the normal route - any memory not used by static values is"]
            #[doc = r" reserved for stack and initialised by cortex-m-rt."]
            #[doc =
            r" To get the same for Core 1, we would need to compile everything seperately and"]
            #[doc =
            r" modify the linker file for both programs, and that's quite annoying."]
            #[doc =
            r" So instead, core1.spawn takes a [usize] which gets used for the stack."]
            #[doc =
            r" NOTE: We use the `Stack` struct here to ensure that it has 32-byte alignment, which allows"]
            #[doc =
            r" the stack guard to take up the least amount of usable RAM."]
            static mut CORE1_STACK : rtic :: export :: Stack < 4096 > = rtic
            :: export :: Stack :: new() ; let mut pac = unsafe
            { rp2040_hal :: pac :: Peripherals :: steal() } ; let mut sio =
            rtic :: export :: Sio :: new(pac.SIO) ; let mut mc = rtic ::
            export :: Multicore ::
            new(& mut pac.PSM, & mut pac.PPB, & mut sio.fifo) ; let cores =
            mc.cores() ; let core1 = & mut cores [1] ; let _ =
            core1.spawn(unsafe { & mut CORE1_STACK.mem }, move ||
            core1_entry()) ; unsafe
            {
                let sio = unsafe { & (* rp2040_hal :: pac :: SIO :: PTR) } ;
                while sio.fifo_st.read().vld().bit()
                { let _ = sio.fifo_rd.read() ; }
                sio.fifo_st.write(| wr | wr.bits(0xff)) ; rp2040_hal :: pac ::
                NVIC ::
                unpend(rp2040_hal :: pac :: Interrupt :: SIO_IRQ_PROC0) ;
                rp2040_hal :: pac :: CorePeripherals ::
                steal().NVIC.set_priority(rp2040_hal :: pac :: Interrupt ::
                SIO_IRQ_PROC0, 0u16 as u8) ; rp2040_hal :: pac :: NVIC ::
                unmask(rp2040_hal :: pac :: Interrupt :: SIO_IRQ_PROC0) ;
            }
        }) ; let mut my_idle_task = MyIdleTask :: init() ; my_idle_task.exec()
        ;
    } #[doc = " ===================================="] #[doc = " CORE 1"]
    #[doc = " ==================================== "] fn init_core1()
    {
        let cpu_id = unsafe
        { pac :: Peripherals :: steal().SIO.cpuid.read().bits() } ; info!
        ("staring core {} ...", cpu_id) ; pac :: NVIC ::
        pend(Interrupt :: TIMER_IRQ_1) ;
    } static mut MY_CORE1_TASK : core :: mem :: MaybeUninit < MyCore1Task > =
    core :: mem :: MaybeUninit :: uninit() ; struct MyCore1Task ; impl
    RticTask for MyCore1Task
    {
        fn init() -> Self { Self } fn exec(& mut self)
        {
            let cpu_id = unsafe
            { pac :: Peripherals :: steal().SIO.cpuid.read().bits() } ; info!
            ("executing task from core {}", cpu_id) ; if let Err(_e) =
            MyCore1SwTask :: spawn(())
            {
                error!
                ("couldn't spawn software task on core {} for the first time",
                cpu_id)
            } let _a = Self :: current_core() ; if let Err(_e) = MyCore1SwTask
            :: spawn(())
            {
                error!
                ("couldn't spawn software task on core {} for the second time",
                cpu_id)
            }
        }
    } impl MyCore1Task { pub const fn priority() -> u16 { 2u16 } } impl
    MyCore1Task
    {
        const fn current_core() -> __rtic__internal__Core1
        { unsafe { __rtic__internal__Core1 :: new() } }
    } static mut MY_CORE1_SW_TASK : core :: mem :: MaybeUninit < MyCore1SwTask
    > = core :: mem :: MaybeUninit :: uninit() ; #[doc = " Software tasks of"]
    #[doc = " Core 1"] struct MyCore1SwTask ; impl RticSwTask for
    MyCore1SwTask
    {
        type SpawnInput = () ; fn init() -> Self { Self } fn
        exec(& mut self, _input : ())
        {
            let cpu_id = unsafe
            { pac :: Peripherals :: steal().SIO.cpuid.read().bits() } ; info!
            ("executing software task from core {}", cpu_id) ;
        }
    } impl MyCore1SwTask { pub const fn priority() -> u16 { 2u16 } } impl
    MyCore1SwTask
    {
        const fn current_core() -> __rtic__internal__Core1
        { unsafe { __rtic__internal__Core1 :: new() } }
    } static mut CROSS_CORE_TASK : core :: mem :: MaybeUninit < CrossCoreTask
    > = core :: mem :: MaybeUninit :: uninit() ; pub struct CrossCoreTask ;
    impl RticSwTask for CrossCoreTask
    {
        type SpawnInput = u32 ; fn init() -> Self { Self } fn
        exec(& mut self, _input : u32)
        {
            let cpu_id = unsafe
            { pac :: Peripherals :: steal().SIO.cpuid.read().bits() } ; info!
            ("executing cross core software task from core {}", cpu_id) ;
        }
    } impl CrossCoreTask { pub const fn priority() -> u16 { 1u16 } } impl
    CrossCoreTask
    {
        const fn current_core() -> __rtic__internal__Core1
        { unsafe { __rtic__internal__Core1 :: new() } }
    } static mut CORE1_PRIORITY2_DISPATCHER : core :: mem :: MaybeUninit <
    Core1Priority2Dispatcher > = core :: mem :: MaybeUninit :: uninit() ;
    #[doc(hidden)] pub struct Core1Priority2Dispatcher ; impl RticTask for
    Core1Priority2Dispatcher
    {
        fn init() -> Self { Self } fn exec(& mut self)
        {
            unsafe
            {
                let mut ready_consumer =
                __rtic_internal__Core1Prio2Tasks__RQ.split().1 ; while let
                Some(task) = ready_consumer.dequeue()
                {
                    match task
                    {
                        Core1Prio2Tasks :: MyCore1SwTask =>
                        {
                            let mut input_consumer =
                            __rtic_internal__MyCore1SwTask__INPUTS.split().1 ; let input
                            = input_consumer.dequeue_unchecked() ;
                            MY_CORE1_SW_TASK.assume_init_mut().exec(input) ;
                        }
                    }
                }
            }
        }
    } impl Core1Priority2Dispatcher
    { pub const fn priority() -> u16 { 2u16 } } impl Core1Priority2Dispatcher
    {
        const fn current_core() -> __rtic__internal__Core1
        { unsafe { __rtic__internal__Core1 :: new() } }
    } static mut CORE1_PRIORITY1_DISPATCHER : core :: mem :: MaybeUninit <
    Core1Priority1Dispatcher > = core :: mem :: MaybeUninit :: uninit() ;
    #[doc(hidden)] pub struct Core1Priority1Dispatcher ; impl RticTask for
    Core1Priority1Dispatcher
    {
        fn init() -> Self { Self } fn exec(& mut self)
        {
            unsafe
            {
                let mut ready_consumer =
                __rtic_internal__Core1Prio1Tasks__RQ.split().1 ; while let
                Some(task) = ready_consumer.dequeue()
                {
                    match task
                    {
                        Core1Prio1Tasks :: CrossCoreTask =>
                        {
                            let mut input_consumer =
                            __rtic_internal__CrossCoreTask__INPUTS.split().1 ; let input
                            = input_consumer.dequeue_unchecked() ;
                            CROSS_CORE_TASK.assume_init_mut().exec(input) ;
                        }
                    }
                }
            }
        }
    } impl Core1Priority1Dispatcher
    { pub const fn priority() -> u16 { 1u16 } } impl Core1Priority1Dispatcher
    {
        const fn current_core() -> __rtic__internal__Core1
        { unsafe { __rtic__internal__Core1 :: new() } }
    } #[allow(non_snake_case)] #[no_mangle] fn TIMER_IRQ_1()
    { unsafe { MY_CORE1_TASK.assume_init_mut().exec() ; } }
    #[allow(non_snake_case)] #[no_mangle] fn I2C1_IRQ()
    { unsafe { CORE1_PRIORITY2_DISPATCHER.assume_init_mut().exec() ; } }
    #[allow(non_snake_case)] #[no_mangle] fn DMA_IRQ_1()
    { unsafe { CORE1_PRIORITY1_DISPATCHER.assume_init_mut().exec() ; } }
    #[doc = "Unique type for core 1"] pub use core1_type_mod ::
    __rtic__internal__Core1 ; mod core1_type_mod
    {
        struct __rtic__internal__Core1Inner ; pub struct
        __rtic__internal__Core1(__rtic__internal__Core1Inner) ; impl
        __rtic__internal__Core1
        {
            pub const unsafe fn new() -> Self
            { __rtic__internal__Core1(__rtic__internal__Core1Inner) }
        }
    } #[doc(hidden)] #[allow(non_upper_case_globals)] const
    __rtic_internal_MASK_CHUNKS_core1 : usize = rtic :: export ::
    compute_mask_chunks([rp2040_hal :: pac :: Interrupt :: TIMER_IRQ_1 as u32,
    rp2040_hal :: pac :: Interrupt :: I2C1_IRQ as u32, rp2040_hal :: pac ::
    Interrupt :: DMA_IRQ_1 as u32,]) ; #[doc(hidden)]
    #[allow(non_upper_case_globals)] const __rtic_internal_MASKS_core1 :
    [rtic :: export :: Mask < __rtic_internal_MASK_CHUNKS_core1 > ; 3] =
    [rtic :: export ::
    create_mask([rp2040_hal :: pac :: Interrupt :: DMA_IRQ_1 as u32,]), rtic
    :: export ::
    create_mask([rp2040_hal :: pac :: Interrupt :: TIMER_IRQ_1 as u32,
    rp2040_hal :: pac :: Interrupt :: I2C1_IRQ as u32,]), rtic :: export ::
    create_mask([]),] ; #[doc = r" Entry of "] #[doc = " CORE 1"] #[no_mangle]
    pub fn core1_entry() ->!
    {
        __rtic_interrupt_free(||
        {
            unsafe
            {
                MY_CORE1_TASK.write(MyCore1Task :: init()) ;
                MY_CORE1_SW_TASK.write(MyCore1SwTask :: init()) ;
                CROSS_CORE_TASK.write(CrossCoreTask :: init()) ;
                CORE1_PRIORITY2_DISPATCHER.write(Core1Priority2Dispatcher ::
                init()) ;
                CORE1_PRIORITY1_DISPATCHER.write(Core1Priority1Dispatcher ::
                init()) ;
            } let shared_resources = init_core1() ; unsafe
            {
                rp2040_hal :: pac :: CorePeripherals ::
                steal().NVIC.set_priority(rp2040_hal :: pac :: Interrupt ::
                TIMER_IRQ_1, 2u16 as u8) ; rp2040_hal :: pac :: NVIC ::
                unmask(rp2040_hal :: pac :: Interrupt :: TIMER_IRQ_1) ;
                rp2040_hal :: pac :: CorePeripherals ::
                steal().NVIC.set_priority(rp2040_hal :: pac :: Interrupt ::
                I2C1_IRQ, 2u16 as u8) ; rp2040_hal :: pac :: NVIC ::
                unmask(rp2040_hal :: pac :: Interrupt :: I2C1_IRQ) ;
                rp2040_hal :: pac :: CorePeripherals ::
                steal().NVIC.set_priority(rp2040_hal :: pac :: Interrupt ::
                DMA_IRQ_1, 1u16 as u8) ; rp2040_hal :: pac :: NVIC ::
                unmask(rp2040_hal :: pac :: Interrupt :: DMA_IRQ_1) ;
            } unsafe
            {
                let sio = unsafe { & (* rp2040_hal :: pac :: SIO :: PTR) } ;
                while sio.fifo_st.read().vld().bit()
                { let _ = sio.fifo_rd.read() ; }
                sio.fifo_st.write(| wr | wr.bits(0xff)) ; rp2040_hal :: pac ::
                NVIC ::
                unpend(rp2040_hal :: pac :: Interrupt :: SIO_IRQ_PROC1) ;
                rp2040_hal :: pac :: CorePeripherals ::
                steal().NVIC.set_priority(rp2040_hal :: pac :: Interrupt ::
                SIO_IRQ_PROC1, 0u16 as u8) ; rp2040_hal :: pac :: NVIC ::
                unmask(rp2040_hal :: pac :: Interrupt :: SIO_IRQ_PROC1) ;
            }
        }) ; loop { unsafe { core :: arch :: asm! ("wfi") ; } }
    }
}