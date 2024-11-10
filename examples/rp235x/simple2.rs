#![no_std]
#![no_main]

use static_cell::StaticCell;

use defmt_rtt as _;
use panic_halt as _;

use rp235x_hal as hal;

use embedded_hal::digital::{OutputPin, StatefulOutputPin};

use embassy_executor::Executor;
use embassy_time::{Duration, Timer};

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32;
const DELAY_MS: u64 = 500u64;

#[embassy_executor::task]
async fn simple() -> ! {
    let mut pac = hal::pac::Peripherals::take().unwrap();

    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let timer = hal::Timer::new_timer1(pac.TIMER1, &mut pac.RESETS, &clocks);

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

#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 5] = [
    hal::binary_info::rp_program_name!(c"rp-hal-async-simple2"),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"Test for embassy Timer"),
    hal::binary_info::rp_program_url!(c"private"),
    hal::binary_info::rp_program_build_attribute!(),
];
