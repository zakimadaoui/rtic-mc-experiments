pub mod my_app
{
    #[doc = r" Include peripheral crate that defines the vector table"] use
    rp2040_hal :: pac as _ ;
    #[doc =
    r" ================================== user includes ===================================="]
    use cortex_m :: asm ; use defmt :: assert_eq ; use defmt :: * ; use
    defmt_rtt as _ ; use panic_probe as _ ; use rp2040_hal :: pac ;
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
    const PING_PONG_DELAY : u32 = 30000000 ;
    #[doc = " Reads core number (0 or 1) from the rp2040 CPUID register"] fn
    get_core_id() -> u32
    { unsafe { (& (* pac :: SIO :: PTR)).cpuid.read().bits() } } static mut
    __rtic_internal__Core0Task__INPUTS : rtic :: export :: Queue < < Core0Task
    as RticSwTask > :: SpawnInput, 2 > = rtic :: export :: Queue :: new() ;
    impl Core0Task
    {
        pub fn
        spawn_from(_spawner : __rtic__internal__Core1, input : < Core0Task as
        RticSwTask > :: SpawnInput) -> Result < (), < Core0Task as RticSwTask
        > :: SpawnInput >
        {
            let mut inputs_producer = unsafe
            { __rtic_internal__Core0Task__INPUTS.split().0 } ; let mut
            ready_producer = unsafe
            { __rtic_internal__Core0Prio1Tasks__RQ.split().0 } ;
            #[doc =
            r" need to protect by a critical section due to many producers of different priorities can spawn/enqueue this task"]
            __rtic_interrupt_free(| | -> Result < (), < Core0Task as
            RticSwTask > :: SpawnInput >
            {
                inputs_producer.enqueue(input) ? ; unsafe
                {
                    ready_producer.enqueue_unchecked(Core0Prio1Tasks ::
                    Core0Task)
                } ;
                __rtic_mc_pend(rp2040_hal :: pac :: Interrupt :: DMA_IRQ_0 as
                u16, 0u32) ; Ok(())
            })
        }
    } #[doc = " Dispatchers of"] #[doc = " Core 0"] #[derive(Clone, Copy)]
    #[doc(hidden)] pub enum Core0Prio1Tasks { Core0Task, } #[doc(hidden)]
    #[allow(non_upper_case_globals)] static mut
    __rtic_internal__Core0Prio1Tasks__RQ : rtic :: export :: Queue <
    Core0Prio1Tasks, 2usize > = rtic :: export :: Queue :: new() ; static mut
    __rtic_internal__Core1Task__INPUTS : rtic :: export :: Queue < < Core1Task
    as RticSwTask > :: SpawnInput, 2 > = rtic :: export :: Queue :: new() ;
    impl Core1Task
    {
        pub fn
        spawn_from(_spawner : __rtic__internal__Core0, input : < Core1Task as
        RticSwTask > :: SpawnInput) -> Result < (), < Core1Task as RticSwTask
        > :: SpawnInput >
        {
            let mut inputs_producer = unsafe
            { __rtic_internal__Core1Task__INPUTS.split().0 } ; let mut
            ready_producer = unsafe
            { __rtic_internal__Core1Prio2Tasks__RQ.split().0 } ;
            #[doc =
            r" need to protect by a critical section due to many producers of different priorities can spawn/enqueue this task"]
            __rtic_interrupt_free(| | -> Result < (), < Core1Task as
            RticSwTask > :: SpawnInput >
            {
                inputs_producer.enqueue(input) ? ; unsafe
                {
                    ready_producer.enqueue_unchecked(Core1Prio2Tasks ::
                    Core1Task)
                } ;
                __rtic_mc_pend(rp2040_hal :: pac :: Interrupt :: DMA_IRQ_1 as
                u16, 1u32) ; Ok(())
            })
        }
    } #[doc = " Dispatchers of"] #[doc = " Core 1"] #[derive(Clone, Copy)]
    #[doc(hidden)] pub enum Core1Prio2Tasks { Core1Task, } #[doc(hidden)]
    #[allow(non_upper_case_globals)] static mut
    __rtic_internal__Core1Prio2Tasks__RQ : rtic :: export :: Queue <
    Core1Prio2Tasks, 2usize > = rtic :: export :: Queue :: new() ;
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
    #[doc = " ==================================== "] fn init_core0()
    {
        assert_eq! (get_core_id(), 0) ; info! ("staring core 0 ...") ; let mut
        device = pac :: Peripherals :: take().unwrap() ; let mut watchdog =
        rp2040_hal :: watchdog :: Watchdog :: new(device.WATCHDOG) ; let
        _clocks = rp2040_hal :: clocks ::
        init_clocks_and_plls(12_000_000u32, device.XOSC, device.CLOCKS,
        device.PLL_SYS, device.PLL_USB, & mut device.RESETS, & mut
        watchdog,).ok().unwrap() ;
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
    } static mut CORE0_TASK : core :: mem :: MaybeUninit < Core0Task > = core
    :: mem :: MaybeUninit :: uninit() ; #[doc = " Software tasks of"]
    #[doc = " Core 0"]
    #[doc = " a Core0 task to be spawned by a task on Core1"] struct Core0Task
    ; impl RticSwTask for Core0Task
    {
        type SpawnInput = u32 ; fn init() -> Self { Self } fn
        exec(& mut self, ping : Self :: SpawnInput)
        {
            assert_eq! (get_core_id(), 0) ; asm :: delay(PING_PONG_DELAY) ;
            let pong = ping + 1 ; info!
            ("CORE0: Got ping {}, sending pong {}", ping, pong) ; if let
            Err(_e) = Core1Task :: spawn_from(Self :: current_core(), pong)
            { error! ("couldn't spawn task on core 1 from core 0") }
        }
    } impl Core0Task { pub const fn priority() -> u16 { 1u16 } } impl
    Core0Task
    {
        const fn current_core() -> __rtic__internal__Core0
        { unsafe { __rtic__internal__Core0 :: new() } }
    } static mut CORE0_PRIORITY1_DISPATCHER : core :: mem :: MaybeUninit <
    Core0Priority1Dispatcher > = core :: mem :: MaybeUninit :: uninit() ;
    #[doc(hidden)] pub struct Core0Priority1Dispatcher ; impl RticTask for
    Core0Priority1Dispatcher
    {
        fn init() -> Self { Self } fn exec(& mut self)
        {
            unsafe
            {
                let mut ready_consumer =
                __rtic_internal__Core0Prio1Tasks__RQ.split().1 ; while let
                Some(task) = ready_consumer.dequeue()
                {
                    match task
                    {
                        Core0Prio1Tasks :: Core0Task =>
                        {
                            let mut input_consumer =
                            __rtic_internal__Core0Task__INPUTS.split().1 ; let input =
                            input_consumer.dequeue_unchecked() ;
                            CORE0_TASK.assume_init_mut().exec(input) ;
                        }
                    }
                }
            }
        }
    } impl Core0Priority1Dispatcher
    { pub const fn priority() -> u16 { 1u16 } } impl Core0Priority1Dispatcher
    {
        const fn current_core() -> __rtic__internal__Core0
        { unsafe { __rtic__internal__Core0 :: new() } }
    } #[allow(non_snake_case)] #[no_mangle] fn DMA_IRQ_0()
    { unsafe { CORE0_PRIORITY1_DISPATCHER.assume_init_mut().exec() ; } }
    #[doc = "Unique type for core 0"] pub use core0_type_mod ::
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
    compute_mask_chunks([rp2040_hal :: pac :: Interrupt :: DMA_IRQ_0 as u32,])
    ; #[doc(hidden)] #[allow(non_upper_case_globals)] const
    __rtic_internal_MASKS_core0 :
    [rtic :: export :: Mask < __rtic_internal_MASK_CHUNKS_core0 > ; 3] =
    [rtic :: export ::
    create_mask([rp2040_hal :: pac :: Interrupt :: DMA_IRQ_0 as u32,]), rtic
    :: export :: create_mask([]), rtic :: export :: create_mask([]),] ;
    #[doc = r" Entry of "] #[doc = " CORE 0"] #[no_mangle] pub fn main() ->!
    {
        __rtic_interrupt_free(||
        {
            unsafe
            {
                CORE0_TASK.write(Core0Task :: init()) ;
                CORE0_PRIORITY1_DISPATCHER.write(Core0Priority1Dispatcher ::
                init()) ;
            } let shared_resources = init_core0() ; unsafe
            {
                rp2040_hal :: pac :: CorePeripherals ::
                steal().NVIC.set_priority(rp2040_hal :: pac :: Interrupt ::
                DMA_IRQ_0, 1u16 as u8) ; rp2040_hal :: pac :: NVIC ::
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
    { assert_eq! (get_core_id(), 1) ; info! ("staring core 1 ...") ; } static
    mut CORE1_TASK : core :: mem :: MaybeUninit < Core1Task > = core :: mem ::
    MaybeUninit :: uninit() ; #[doc = " Software tasks of"] #[doc = " Core 1"]
    #[doc = " a Core1 task to be spawned by a task on Core0"] struct Core1Task
    ; impl RticSwTask for Core1Task
    {
        type SpawnInput = u32 ; fn init() -> Self
        {
            Core0Task ::
            spawn_from(Self :: current_core(),
            1).expect("Couldn't start task on core 0") ; Self
        } fn exec(& mut self, pong : Self :: SpawnInput)
        {
            assert_eq! (get_core_id(), 1) ; asm :: delay(PING_PONG_DELAY) ;
            let ping = pong + 1 ; info!
            ("CORE1: Got pong {}, sending ping {}", pong, ping) ; if let
            Err(_e) = Core0Task :: spawn_from(Self :: current_core(), ping)
            { error! ("couldn't spawn task on core 0 from core 1") }
        }
    } impl Core1Task { pub const fn priority() -> u16 { 2u16 } } impl
    Core1Task
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
                        Core1Prio2Tasks :: Core1Task =>
                        {
                            let mut input_consumer =
                            __rtic_internal__Core1Task__INPUTS.split().1 ; let input =
                            input_consumer.dequeue_unchecked() ;
                            CORE1_TASK.assume_init_mut().exec(input) ;
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
    } #[allow(non_snake_case)] #[no_mangle] fn DMA_IRQ_1()
    { unsafe { CORE1_PRIORITY2_DISPATCHER.assume_init_mut().exec() ; } }
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
    compute_mask_chunks([rp2040_hal :: pac :: Interrupt :: DMA_IRQ_1 as u32,])
    ; #[doc(hidden)] #[allow(non_upper_case_globals)] const
    __rtic_internal_MASKS_core1 :
    [rtic :: export :: Mask < __rtic_internal_MASK_CHUNKS_core1 > ; 3] =
    [rtic :: export :: create_mask([]), rtic :: export ::
    create_mask([rp2040_hal :: pac :: Interrupt :: DMA_IRQ_1 as u32,]), rtic
    :: export :: create_mask([]),] ; #[doc = r" Entry of "] #[doc = " CORE 1"]
    #[no_mangle] pub fn core1_entry() ->!
    {
        __rtic_interrupt_free(||
        {
            unsafe
            {
                CORE1_TASK.write(Core1Task :: init()) ;
                CORE1_PRIORITY2_DISPATCHER.write(Core1Priority2Dispatcher ::
                init()) ;
            } let shared_resources = init_core1() ; unsafe
            {
                rp2040_hal :: pac :: CorePeripherals ::
                steal().NVIC.set_priority(rp2040_hal :: pac :: Interrupt ::
                DMA_IRQ_1, 2u16 as u8) ; rp2040_hal :: pac :: NVIC ::
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