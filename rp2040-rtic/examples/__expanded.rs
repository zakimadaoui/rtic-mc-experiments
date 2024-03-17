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
    rp2040_hal :: pac :: { self } ;
    #[doc =
    r" ==================================== init task ======================================"]
    fn system_init() -> SharedResources
    {
        let mut device = pac :: Peripherals :: take().unwrap() ; let mut
        watchdog = rp2040_hal :: watchdog :: Watchdog :: new(device.WATCHDOG)
        ; let clocks = rp2040_hal :: clocks ::
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
        alarm0.enable_interrupt() ; unsafe
        {
            pac :: NVIC :: unmask(pac :: Interrupt :: TIMER_IRQ_0) ; pac ::
            NVIC :: unmask(pac :: Interrupt :: TIMER_IRQ_1) ; pac :: NVIC ::
            unmask(pac :: Interrupt :: TIMER_IRQ_2) ;
        } SharedResources { alarm : alarm0, led : led_pin, }
    }
    #[doc =
    r" ==================================== idle task ======================================"]
    static mut MY_IDLE_TASK : core :: mem :: MaybeUninit < MyIdleTask > = core
    :: mem :: MaybeUninit :: uninit() ; struct MyIdleTask { count : u32, }
    impl RticIdleTask for MyIdleTask
    {
        fn init() -> Self { Self { count : 0 } } fn exec(& mut self) ->!
        {
            loop
            {
                self.count += 1 ; info! ("looping in idle... {}", self.count)
                ; asm :: delay(120000000) ;
            }
        }
    } impl MyIdleTask { pub const fn priority() -> u16 { 0u16 } }
    #[doc =
    r" ======================== define static mut shared resources ========================="]
    static mut SHARED_RESOURCES : core :: mem :: MaybeUninit < SharedResources
    > = core :: mem :: MaybeUninit :: uninit() ; struct SharedResources
    { alarm : Alarm0, led : LedOutPin, }
    #[doc =
    r"====================== proxies for accessing the shared resources ===================="]
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
                __rtic_internal_MASKS, f) ;
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
            const CEILING : u16 = 1u16 ; let task_priority = self.priority ;
            let resource = unsafe
            { & mut SHARED_RESOURCES.assume_init_mut().led } as * mut _ ;
            unsafe
            {
                rtic :: export ::
                lock(resource, task_priority, CEILING, &
                __rtic_internal_MASKS, f) ;
            }
        }
    }
    #[doc =
    r"======================== define and bind hw tasks to interrupts ======================"]
    static mut MY_TASK : core :: mem :: MaybeUninit < MyTask > = core :: mem
    :: MaybeUninit :: uninit() ; struct MyTask { is_high : bool, } impl
    RticTask for MyTask
    {
        fn init() -> Self { Self { is_high : false } } fn exec(& mut self)
        {
            self.shared().led.lock(| led_pin |
            {
                if self.is_high
                { let _ = led_pin.set_low() ; self.is_high = false ; } else
                { let _ = led_pin.set_high() ; self.is_high = true ; }
            }) ;
            self.shared().alarm.lock(| alarm0 |
            {
                let _ = alarm0.schedule(MicrosDurationU32 :: millis(DELAY)) ;
                alarm0.clear_interrupt() ;
            }) ;
        }
    } impl MyTask { pub const fn priority() -> u16 { 1u16 } } impl MyTask
    {
        pub fn shared(& self) -> __my_task_shared_resources
        {
            const TASK_PRIORITY : u16 = 1u16 ; __my_task_shared_resources ::
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
    } static mut MY_TASK3 : core :: mem :: MaybeUninit < MyTask3 > = core ::
    mem :: MaybeUninit :: uninit() ; struct MyTask3 ; impl RticTask for
    MyTask3 { fn init() -> Self { Self } fn exec(& mut self) {} } impl MyTask3
    { pub const fn priority() -> u16 { 3u16 } } impl MyTask3
    {
        pub fn shared(& self) -> __my_task3_shared_resources
        {
            const TASK_PRIORITY : u16 = 3u16 ; __my_task3_shared_resources ::
            new(TASK_PRIORITY)
        }
    } struct __my_task3_shared_resources { pub alarm : __alarm_mutex, } impl
    __my_task3_shared_resources
    {
        #[inline(always)] pub fn new(priority : u16) -> Self
        { Self { alarm : __alarm_mutex :: new(priority), } }
    } #[allow(non_snake_case)] #[no_mangle] fn TIMER_IRQ_0()
    { unsafe { MY_TASK.assume_init_mut().exec() ; } } #[allow(non_snake_case)]
    #[no_mangle] fn TIMER_IRQ_2()
    { unsafe { MY_TASK3.assume_init_mut().exec() ; } }
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
    r" ======================================= main ========================================"]
    #[no_mangle] pub fn main() ->!
    {
        unsafe { core :: arch :: asm! ("cpsid i") ; } unsafe
        {
            MY_TASK.write(MyTask :: init()) ;
            MY_TASK3.write(MyTask3 :: init()) ;
        } unsafe { SHARED_RESOURCES.write(system_init()) ; } unsafe
        { core :: arch :: asm! ("cpsie i") ; } let mut my_idle_task =
        MyIdleTask :: init() ; my_idle_task.exec() ;
    } #[doc = r" user code"] type LedOutPin = Pin < Gpio25, FunctionSio <
    SioOutput >, PullDown > ; static DELAY : u32 = 100 ;
    #[doc = r" Computed priority Masks"] #[doc(hidden)]
    #[allow(non_upper_case_globals)] const __rtic_internal_MASK_CHUNKS : usize
    = rtic :: export ::
    compute_mask_chunks([rp2040_hal :: pac :: Interrupt :: TIMER_IRQ_0 as u32,
    rp2040_hal :: pac :: Interrupt :: TIMER_IRQ_2 as u32,]) ; #[doc(hidden)]
    #[allow(non_upper_case_globals)] const __rtic_internal_MASKS :
    [rtic :: export :: Mask < __rtic_internal_MASK_CHUNKS > ; 3] =
    [rtic :: export ::
    create_mask([rp2040_hal :: pac :: Interrupt :: TIMER_IRQ_0 as u32,]), rtic
    :: export :: create_mask([]), rtic :: export ::
    create_mask([rp2040_hal :: pac :: Interrupt :: TIMER_IRQ_2 as u32,]),] ;
}