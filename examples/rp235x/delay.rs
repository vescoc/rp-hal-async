#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_halt as _;

use embedded_hal::digital::OutputPin;
use embedded_hal_async::delay::DelayNs;

use rp_hal_async::IntoAsync;

use rp235x_hal as hal;

use static_cell::StaticCell;

use embassy_executor::Executor;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32;
const DELAY_MS: u32 = 2_000u32;

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

    unsafe {
        rp_hal_async::init();
    }

    let mut timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);
    let mut alarm = timer.alarm_1().unwrap().into_async();

    defmt::info!("rp-hal-async-simple1");

    let sio = hal::Sio::new(pac.SIO);

    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led_pin = pins.gpio25.into_push_pull_output();

    loop {
        defmt::trace!("tick 1");
        led_pin.set_high().unwrap();
        alarm.delay_ms(DELAY_MS).await;

        defmt::trace!("tick 2");
        led_pin.set_low().unwrap();
        alarm.delay_ms(DELAY_MS).await;
    }
}

#[hal::entry]
fn main() -> ! {
    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    let executor = EXECUTOR.init(Executor::new());

    executor.run(|spawner| spawner.spawn(simple()).unwrap());
}

#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 5] = [
    hal::binary_info::rp_program_name!(c"rp-hal-async-simple1"),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"Test for embedded-hal-async / timer Delay"),
    hal::binary_info::rp_program_url!(c"private"),
    hal::binary_info::rp_program_build_attribute!(),
];
