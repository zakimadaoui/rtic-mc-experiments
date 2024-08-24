#![no_std]
#![no_main]

#[allow(unused)]
use defmt::*;
use defmt_rtt as _;
use panic_probe as _;
pub mod app {
    /// ================================== user includes ====================================
    use core::sync::atomic::{AtomicU32, Ordering};
    use embedded_hal::digital::v2::ToggleableOutputPin;
    use fugit::{MicrosDurationU32, RateExtU32};
    use heapless::String;
    use rp2040_hal::gpio::bank0::{Gpio0, Gpio1, Gpio25};
    use rp2040_hal::gpio::{FunctionSio, FunctionUart, Pin, PullDown, SioOutput};
    use rp2040_hal::pac;
    use rp2040_hal::timer::{Alarm, Alarm0};
    use rp2040_hal::uart::{
        DataBits, Reader as UartReader, StopBits, UartConfig, UartPeripheral, Writer,
    };
    use rp2040_hal::Clock;
    use rp_pico;
    /// Include peripheral crate that defines the vector table
    use rp_pico::hal::pac as _;
    /// ==================================== rtic traits ====================================
    pub use rtic_traits::*;
    /// Module defining rtic traits
    mod rtic_traits {
        /// Trait for a hardware task
        pub trait RticTask {
            /// Associated type that can be used to make [Self::init] take arguments
            type InitArgs;
            /// Task local variables initialization routine
            fn init(args: Self::InitArgs) -> Self;
            /// Function to be bound to a HW Interrupt
            fn exec(&mut self);
        }
        /// Trait for an idle task
        pub trait RticIdleTask {
            /// Associated type that can be used to make [Self::init] take arguments
            type InitArgs;
            /// Task local variables initialization routine
            fn init(args: Self::InitArgs) -> Self;
            /// Function to be executing when no other task is running
            fn exec(&mut self) -> !;
        }
        pub trait RticMutex {
            type ResourceType;
            fn lock(&mut self, f: impl FnOnce(&mut Self::ResourceType));
        }
    }
    /// ================================== rtic functions ===================================
    /// critical section function
    #[inline]
    pub fn __rtic_interrupt_free<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        unsafe {
            asm!("cpsid i");
        }
        let r = f();
        unsafe {
            asm!("cpsie i");
        }
        r
    }
    /// ==================================== User code ======================================
    /// External high-speed crystal on the Raspberry Pi Pico board is 12 MHz. Adjust
    /// if your board has a different frequency
    const XTAL_FREQ_HZ: u32 = 12_000_000u32;
    static TARGET_DURATION: AtomicU32 = AtomicU32::new(0);
    static TARGET_TICKS: AtomicU32 = AtomicU32::new(0);
    type UartRx = UartReader<
        pac::UART0,
        (
            Pin<Gpio0, FunctionUart, PullDown>,
            Pin<Gpio1, FunctionUart, PullDown>,
        ),
    >;
    type UartTx = Writer<
        pac::UART0,
        (
            Pin<Gpio0, FunctionUart, PullDown>,
            Pin<Gpio1, FunctionUart, PullDown>,
        ),
    >;
    enum Command {
        Blink,
        Unknown,
    }
    impl CommandReceiverTask {
        fn run_command(&mut self) {
            match self.command {
                Command::Blink => {
                    let (blinks, duration) = self.data.split_once(' ').unwrap_or(("0", "0"));
                    let blinks: u32 = blinks.parse().unwrap_or(0);
                    let duration: u32 = duration.parse().unwrap_or(0);
                    TARGET_TICKS.store(blinks, Ordering::SeqCst);
                    TARGET_DURATION.store(duration, Ordering::SeqCst);
                    self.shared()
                        .uart_tx
                        .lock(|uart| uart.write_full_blocking(b"Starting blinky ...\r\n"));
                    self.shared().alarm.lock(|alarm| {
                        let _ = alarm.schedule(MicrosDurationU32::millis(duration));
                    });
                }
                Command::Unknown => self
                    .shared()
                    .uart_tx
                    .lock(|uart| uart.write_full_blocking(b"Unknown command !\r\n")),
            }
        }
    }
    type LedPin = Pin<Gpio25, FunctionSio<SioOutput>, PullDown>;
    /// Software tasks of
    /// Core 0
    /// Dispatchers of
    /// Core 0
    /// RTIC Software task trait
    /// Trait for a software task
    pub trait RticSwTask {
        type InitArgs;
        type SpawnInput;
        /// Task local variables initialization routine
        fn init(args: Self::InitArgs) -> Self;
        /// Function to be executing when the scheduled software task is dispatched
        fn exec(&mut self, input: Self::SpawnInput);
    }
    /// Core local interrupt pending
    #[doc(hidden)]
    #[inline]
    pub fn __rtic_local_irq_pend(irq_nbr: u16) {
        unsafe {
            (*rtic::export::NVIC::PTR).ispr[usize::from(irq_nbr / 32)].write(1 << (irq_nbr % 32))
        }
    }
    #[doc(hidden)]
    #[inline]
    pub fn __rtic_cross_irq_pend(irq_nbr: u16, core: u32) {
        rtic::export::cross_core::pend_irq(irq_nbr);
    }
    /// ====================================
    /// CORE 0
    /// ====================================
    static mut SHARED: core::mem::MaybeUninit<Shared> = core::mem::MaybeUninit::uninit();
    struct Shared {
        uart_tx: UartTx,
        alarm: Alarm0,
        target_blinks: u32,
    }
    fn init() -> (Shared, TaskInits) {
        let mut device = pac::Peripherals::take().unwrap();
        let mut watchdog = rp2040_hal::watchdog::Watchdog::new(device.WATCHDOG);
        let clocks = rp2040_hal::clocks::init_clocks_and_plls(
            XTAL_FREQ_HZ,
            device.XOSC,
            device.CLOCKS,
            device.PLL_SYS,
            device.PLL_USB,
            &mut device.RESETS,
            &mut watchdog,
        )
        .ok()
        .unwrap();
        let sio = rp2040_hal::Sio::new(device.SIO);
        let pins = rp2040_hal::gpio::Pins::new(
            device.IO_BANK0,
            device.PADS_BANK0,
            sio.gpio_bank0,
            &mut device.RESETS,
        );
        let uart_pins = (pins.gpio0.into_function(), pins.gpio1.into_function());
        let uart = UartPeripheral::new(device.UART0, uart_pins, &mut device.RESETS)
            .enable(
                UartConfig::new(115200.Hz(), DataBits::Eight, None, StopBits::One),
                clocks.peripheral_clock.freq(),
            )
            .unwrap();
        let (mut uart_rx, mut uart_tx) = uart.split();
        uart_rx.enable_rx_interrupt();
        uart_tx.disable_tx_interrupt();
        unsafe { pac::NVIC::unmask(pac::Interrupt::UART0_IRQ) };
        let led_pin = pins.gpio25.into_push_pull_output();
        let mut timer = rp2040_hal::Timer::new(device.TIMER, &mut device.RESETS, &clocks);
        let mut alarm0 = timer.alarm_0().unwrap();
        alarm0.enable_interrupt();
        unsafe { pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0) };
        uart_tx.write_full_blocking(b"Welcome to LedCommander Example\r\n");
        uart_tx
            .write_full_blocking(
                b"Enter the command and its arguments: <cmd> <arg1 arg2 ... arg_n>. Possible commands are:\r\n",
            );
        uart_tx
            .write_full_blocking(
                b"b <count> <duration> # toggles an led <count> times with <duration> between each toggle.\r\n",
            );
        (
            Shared {
                uart_tx,
                alarm: alarm0,
                target_blinks: 0,
            },
            TaskInits {
                command_receiver_task: CommandReceiverTask::init(uart_rx),
                command_executor_task: CommandExecutorTask::init(led_pin),
            },
        )
    }
    static mut COMMAND_RECEIVER_TASK: core::mem::MaybeUninit<CommandReceiverTask> =
        core::mem::MaybeUninit::uninit();
    /// Task the receives commands to blink the led
    struct CommandReceiverTask {
        data: heapless::String<30>,
        command: Command,
        read_command: bool,
        uart_rx: UartRx,
    }
    impl RticTask for CommandReceiverTask {
        type InitArgs = UartRx;
        fn init(uart_rx: UartRx) -> Self {
            Self {
                data: String::new(),
                read_command: true,
                command: Command::Unknown,
                uart_rx,
            }
        }
        fn exec(&mut self) {
            let mut data = [0_u8; 48];
            let bytes = self.uart_rx.read_raw(&mut data).unwrap();
            self.shared()
                .uart_tx
                .lock(|uart| uart.write_full_blocking(&data[..bytes]));
            for b in &data[..bytes] {
                if self.read_command {
                    let cmd = match b {
                        b'b' => Command::Blink,
                        _ => Command::Unknown,
                    };
                    self.command = cmd;
                    self.read_command = false;
                } else if (b == &b'\n') || (b == &b'\r') {
                    self.run_command();
                    self.read_command = true;
                    self.data.clear();
                    self.command = Command::Unknown;
                } else {
                    if *b != b' ' || !self.data.is_empty() {
                        let _ = self.data.push(*b as char);
                    }
                }
            }
        }
    }
    impl CommandReceiverTask {
        pub const fn priority() -> u16 {
            1u16
        }
    }
    impl CommandReceiverTask {
        pub fn shared(&self) -> __command_receiver_task_shared_resources {
            const TASK_PRIORITY: u16 = 1u16;
            __command_receiver_task_shared_resources::new(TASK_PRIORITY)
        }
    }
    struct __command_receiver_task_shared_resources {
        pub uart_tx: __uart_tx_mutex,
        pub alarm: __alarm_mutex,
    }
    impl __command_receiver_task_shared_resources {
        #[inline(always)]
        pub fn new(priority: u16) -> Self {
            Self {
                uart_tx: __uart_tx_mutex::new(priority),
                alarm: __alarm_mutex::new(priority),
            }
        }
    }
    impl CommandReceiverTask {
        const fn current_core() -> __rtic__internal__Core0 {
            unsafe { __rtic__internal__Core0::new() }
        }
    }
    static mut COMMAND_EXECUTOR_TASK: core::mem::MaybeUninit<CommandExecutorTask> =
        core::mem::MaybeUninit::uninit();
    /// Task that blinks the rp-pico onboard LED and that send a message "LED ON!" and "LED OFF!" do USB-Serial.
    pub struct CommandExecutorTask {
        led: LedPin,
    }
    impl RticTask for CommandExecutorTask {
        type InitArgs = LedPin;
        fn init(led: LedPin) -> Self {
            Self { led }
        }
        fn exec(&mut self) {
            let duration = TARGET_DURATION.load(Ordering::SeqCst);
            let blinks_left = TARGET_TICKS.load(Ordering::SeqCst);
            let blinks_left = blinks_left.saturating_sub(1);
            TARGET_TICKS.store(blinks_left, Ordering::SeqCst);
            let _ = self.led.toggle();
            if blinks_left == 0 {
                self.shared()
                    .uart_tx
                    .lock(|uart| uart.write_full_blocking(b"finished pattern !\r\n"));
            }
            self.shared().alarm.lock(|alarm0| {
                if blinks_left != 0 {
                    let _ = alarm0.schedule(MicrosDurationU32::millis(duration));
                }
                alarm0.clear_interrupt();
            });
        }
    }
    impl CommandExecutorTask {
        pub const fn priority() -> u16 {
            2u16
        }
    }
    impl CommandExecutorTask {
        pub fn shared(&self) -> __command_executor_task_shared_resources {
            const TASK_PRIORITY: u16 = 2u16;
            __command_executor_task_shared_resources::new(TASK_PRIORITY)
        }
    }
    struct __command_executor_task_shared_resources {
        pub uart_tx: __uart_tx_mutex,
        pub alarm: __alarm_mutex,
        pub target_blinks: __target_blinks_mutex,
    }
    impl __command_executor_task_shared_resources {
        #[inline(always)]
        pub fn new(priority: u16) -> Self {
            Self {
                uart_tx: __uart_tx_mutex::new(priority),
                alarm: __alarm_mutex::new(priority),
                target_blinks: __target_blinks_mutex::new(priority),
            }
        }
    }
    impl CommandExecutorTask {
        const fn current_core() -> __rtic__internal__Core0 {
            unsafe { __rtic__internal__Core0::new() }
        }
    }
    #[allow(non_snake_case)]
    #[no_mangle]
    fn UART0_IRQ() {
        unsafe { COMMAND_RECEIVER_TASK.assume_init_mut().exec() };
    }
    #[allow(non_snake_case)]
    #[no_mangle]
    fn TIMER_IRQ_0() {
        unsafe { COMMAND_EXECUTOR_TASK.assume_init_mut().exec() };
    }
    struct __uart_tx_mutex {
        #[doc(hidden)]
        task_priority: u16,
    }
    impl __uart_tx_mutex {
        #[inline(always)]
        pub fn new(task_priority: u16) -> Self {
            Self { task_priority }
        }
    }
    impl RticMutex for __uart_tx_mutex {
        type ResourceType = UartTx;
        fn lock(&mut self, f: impl FnOnce(&mut Self::ResourceType)) {
            const CEILING: u16 = 2u16;
            let task_priority = self.task_priority;
            let resource_ptr = unsafe { &mut SHARED.assume_init_mut().uart_tx } as *mut _;
            unsafe {
                rtic::export::lock(
                    resource_ptr,
                    task_priority,
                    CEILING,
                    &__rtic_internal_MASKS_core0,
                    f,
                );
            }
        }
    }
    struct __alarm_mutex {
        #[doc(hidden)]
        task_priority: u16,
    }
    impl __alarm_mutex {
        #[inline(always)]
        pub fn new(task_priority: u16) -> Self {
            Self { task_priority }
        }
    }
    impl RticMutex for __alarm_mutex {
        type ResourceType = Alarm0;
        fn lock(&mut self, f: impl FnOnce(&mut Self::ResourceType)) {
            const CEILING: u16 = 2u16;
            let task_priority = self.task_priority;
            let resource_ptr = unsafe { &mut SHARED.assume_init_mut().alarm } as *mut _;
            unsafe {
                rtic::export::lock(
                    resource_ptr,
                    task_priority,
                    CEILING,
                    &__rtic_internal_MASKS_core0,
                    f,
                );
            }
        }
    }
    struct __target_blinks_mutex {
        #[doc(hidden)]
        task_priority: u16,
    }
    impl __target_blinks_mutex {
        #[inline(always)]
        pub fn new(task_priority: u16) -> Self {
            Self { task_priority }
        }
    }
    impl RticMutex for __target_blinks_mutex {
        type ResourceType = u32;
        fn lock(&mut self, f: impl FnOnce(&mut Self::ResourceType)) {
            const CEILING: u16 = 2u16;
            let task_priority = self.task_priority;
            let resource_ptr = unsafe { &mut SHARED.assume_init_mut().target_blinks } as *mut _;
            unsafe {
                rtic::export::lock(
                    resource_ptr,
                    task_priority,
                    CEILING,
                    &__rtic_internal_MASKS_core0,
                    f,
                );
            }
        }
    }
    ///Unique type for core 0
    pub use core0_type_mod::__rtic__internal__Core0;
    mod core0_type_mod {
        struct __rtic__internal__Core0Inner;
        pub struct __rtic__internal__Core0(__rtic__internal__Core0Inner);
        impl __rtic__internal__Core0 {
            pub const unsafe fn new() -> Self {
                __rtic__internal__Core0(__rtic__internal__Core0Inner)
            }
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals)]
    const __rtic_internal_MASK_CHUNKS_core0: usize = rtic::export::compute_mask_chunks([
        rp_pico::hal::pac::Interrupt::UART0_IRQ as u32,
        rp_pico::hal::pac::Interrupt::TIMER_IRQ_0 as u32,
    ]);
    #[doc(hidden)]
    #[allow(non_upper_case_globals)]
    const __rtic_internal_MASKS_core0: [rtic::export::Mask<__rtic_internal_MASK_CHUNKS_core0>; 3] = [
        rtic::export::create_mask([rp_pico::hal::pac::Interrupt::UART0_IRQ as u32]),
        rtic::export::create_mask([rp_pico::hal::pac::Interrupt::TIMER_IRQ_0 as u32]),
        rtic::export::create_mask([]),
    ];
    /// Type representing tasks that need explicit user initialization
    pub struct TaskInits {
        pub command_receiver_task: CommandReceiverTask,
        pub command_executor_task: CommandExecutorTask,
    }
    /// Entry of
    /// CORE 0
    #[no_mangle]
    pub fn main() -> ! {
        __rtic_interrupt_free(|| {
            let (__shared_resources, __late_task_inits): (Shared, TaskInits) = init();
            unsafe {
                SHARED.write(__shared_resources);
            }
            unsafe {
                COMMAND_RECEIVER_TASK.write(__late_task_inits.command_receiver_task);
                COMMAND_EXECUTOR_TASK.write(__late_task_inits.command_executor_task);
            }
            unsafe {}
            unsafe {
                rp_pico::hal::pac::CorePeripherals::steal()
                    .NVIC
                    .set_priority(rp_pico::hal::pac::Interrupt::UART0_IRQ, 1u16 as u8);
                rp_pico::hal::pac::NVIC::unmask(rp_pico::hal::pac::Interrupt::UART0_IRQ);
                rp_pico::hal::pac::CorePeripherals::steal()
                    .NVIC
                    .set_priority(rp_pico::hal::pac::Interrupt::TIMER_IRQ_0, 2u16 as u8);
                rp_pico::hal::pac::NVIC::unmask(rp_pico::hal::pac::Interrupt::TIMER_IRQ_0);
            }
        });
        loop {
            unsafe {
                asm!("wfi");
            }
        }
    }
    use core::arch::asm;
}
