pub mod rtic_app {
    use cortex_m::asm;
    use defmt::*;
    use defmt_rtt as _;
    use embedded_hal::digital::v2::OutputPin;
    use panic_probe as _;
    use rp2040_hal::fugit::MicrosDurationU32;
    use rp2040_hal::gpio::bank0::Gpio25;
    use rp2040_hal::gpio::{FunctionSio, Pin, PullDown, SioOutput};
    use rp2040_hal::pac::{self};
    use rp2040_hal::timer::{Alarm, Alarm0};
    type LedOutPin = Pin<Gpio25, FunctionSio<SioOutput>, PullDown>;
    static DELAY: u32 = 100;
    #[shared]
    struct SharedResources {
        alarm: Alarm0,
        led: LedOutPin,
    }
    #[init]
    fn system_init() -> SharedResources {
        let mut device = pac::Peripherals::take().unwrap();
        let mut watchdog = rp2040_hal::watchdog::Watchdog::new(device.WATCHDOG);
        let clocks = rp2040_hal::clocks::init_clocks_and_plls(
            12_000_000u32,
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
        let led_pin = pins.gpio25.into_push_pull_output();
        let mut timer = rp2040_hal::Timer::new(device.TIMER, &mut device.RESETS, &clocks);
        let mut alarm0 = timer.alarm_0().unwrap();
        alarm0.schedule(MicrosDurationU32::millis(DELAY)).unwrap();
        alarm0.enable_interrupt();
        unsafe {
            pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0);
            pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_1);
            pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_2);
        };
        SharedResources {
            alarm: alarm0,
            led: led_pin,
        }
    }
    # [task (binds = TIMER_IRQ_0 , priority = 1 , shared = [alarm , led])]
    struct MyTask {
        is_high: bool,
    }
    impl RticTask for MyTask {
        fn init() -> Self {
            Self { is_high: false }
        }
        fn exec(&mut self) {
            self.shared().led.lock(|led_pin| {
                if self.is_high {
                    let _ = led_pin.set_low();
                    self.is_high = false;
                } else {
                    let _ = led_pin.set_high();
                    self.is_high = true;
                }
            });
            self.shared().alarm.lock(|alarm0| {
                let _ = alarm0.schedule(MicrosDurationU32::millis(DELAY));
                alarm0.clear_interrupt();
            });
        }
    }
    # [task (binds = TIMER_IRQ_2 , priority = 3 , shared = [alarm])]
    struct MyTask3;
    impl RticTask for MyTask3 {
        fn init() -> Self {
            Self
        }
        fn exec(&mut self) {}
    }
    #[idle]
    struct MyIdleTask {
        count: u32,
    }
    impl RticIdleTask for MyIdleTask {
        fn init() -> Self {
            Self { count: 0 }
        }
        fn exec(&mut self) -> ! {
            loop {
                self.count += 1;
                info!("looping in idle... {}", self.count);
                asm::delay(120000000);
            }
        }
    }
    #[doc = r" ============================= Software-pass content ===================================="]
    # [task (priority = 2 , shared = [led])]
    struct MyTask2;
    impl RticSwTask for MyTask2 {
        type SpawnInput = u8;
        fn init() -> Self {
            Self
        }
        fn exec(&mut self, _input: u8) {
            self.shared().led.lock(|_led| {})
        }
    }
    static mut __rtic_internal__MyTask2__INPUTS: rtic::export::Queue<
        <MyTask2 as RticSwTask>::SpawnInput,
        2,
    > = rtic::export::Queue::new();
    impl MyTask2 {
        pub fn spawn(
            input: <MyTask2 as RticSwTask>::SpawnInput,
        ) -> Result<(), <MyTask2 as RticSwTask>::SpawnInput> {
            Ok(())
        }
    }
    # [task (priority = 2 , shared = [led])]
    struct MyTask7;
    impl RticSwTask for MyTask7 {
        type SpawnInput = u8;
        fn init() -> Self {
            Self
        }
        fn exec(&mut self, _input: u8) {}
    }
    static mut __rtic_internal__MyTask7__INPUTS: rtic::export::Queue<
        <MyTask7 as RticSwTask>::SpawnInput,
        2,
    > = rtic::export::Queue::new();
    impl MyTask7 {
        pub fn spawn(
            input: <MyTask7 as RticSwTask>::SpawnInput,
        ) -> Result<(), <MyTask7 as RticSwTask>::SpawnInput> {
            Ok(())
        }
    }
    #[derive(Clone, Copy)]
    #[doc(hidden)]
    pub enum Prio2Tasks {
        MyTask2,
        MyTask7,
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals)]
    static mut __rtic_internal__Prio2Tasks__RQ: rtic::export::Queue<Prio2Tasks, 3usize> =
        rtic::export::Queue::new();
    #[doc(hidden)]
    # [task (binds = DMA_IRQ_0 , priority = "2")]
    pub struct Priority2Dispatcher;
    impl RticTask for Priority2Dispatcher {
        fn init() -> Self {
            Self
        }
        fn exec(&mut self) {
            unsafe {
                let mut ready_consumer = __rtic_internal__Prio2Tasks__RQ.split().1;
                while let Some(task) = ready_consumer.dequeue() {
                    match task {
                        Prio2Tasks::MyTask2 => {
                            let mut input_consumer = __rtic_internal__MyTask2__INPUTS.split().1;
                            let input = input_consumer.dequeue_unchecked();
                            MY_TASK2.assume_init_mut().exec(input);
                        }
                        Prio2Tasks::MyTask7 => {
                            let mut input_consumer = __rtic_internal__MyTask7__INPUTS.split().1;
                            let input = input_consumer.dequeue_unchecked();
                            MY_TASK7.assume_init_mut().exec(input);
                        }
                    }
                }
            }
        }
    }
    #[doc = r" Trait for an idle task"]
    pub trait RticSwTask {
        type SpawnInput;
        #[doc = r" Task local variables initialization routine"]
        fn init() -> Self;
        #[doc = r" Function to be executing when the scheduled software task is dispatched"]
        fn exec(&mut self, input: Self::SpawnInput);
    }
}
