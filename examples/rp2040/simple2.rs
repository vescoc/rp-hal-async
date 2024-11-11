#![no_std]
#![no_main]

use static_cell::StaticCell;

use defmt_rtt as _;
use panic_halt as _;

use rp_pico::hal;

use embedded_hal::digital::{OutputPin, StatefulOutputPin};

use embassy_executor::Executor;
use embassy_time::{Duration, Timer};

const DELAY_MS: u64 = 500u64;

#[embassy_executor::task]
async fn simple() -> ! {
    let mut pac = hal::pac::Peripherals::take().unwrap();

    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    unsafe {
        rp_hal_async::init(timer);
    }

    defmt::info!("rp-hal-async-simple2");

    let sio = hal::Sio::new(pac.SIO);

    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led_pin = pins.gpio25.into_push_pull_output();

    led_pin.set_high().unwrap();
    loop {
        defmt::trace!("tick");
        Timer::after(Duration::from_millis(DELAY_MS)).await;
        defmt::trace!("done wait");
        led_pin.toggle().unwrap();
    }
}

#[hal::entry]
fn main() -> ! {
    static EXECUTOR: StaticCell<Executor> = StaticCell::new();

    defmt::trace!("main");
    let executor = EXECUTOR.init(Executor::new());

    executor.run(|spawner| spawner.spawn(simple()).unwrap());
}
