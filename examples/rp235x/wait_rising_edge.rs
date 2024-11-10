#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_halt as _;

use embedded_hal::digital::OutputPin;
use embedded_hal_async::digital::Wait;

use rp_hal_async::IntoAsync;

use rp235x_hal as hal;

use static_cell::StaticCell;

use embassy_executor::Executor;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

#[embassy_executor::task]
async fn simple() -> ! {
    let mut pac = hal::pac::Peripherals::take().unwrap();

    unsafe {
        rp_hal_async::init();
    }

    defmt::info!("rp-hal-async-wait");

    let sio = hal::Sio::new(pac.SIO);

    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led_pin = pins.gpio25.into_push_pull_output();
    let mut input_pin = pins.gpio0.into_pull_up_input().into_async();

    loop {
        defmt::trace!("high");
        led_pin.set_high().unwrap();

        defmt::trace!("wait for rising edge (on high)");
        input_pin.wait_for_rising_edge().await.unwrap();

        defmt::trace!("low");
        led_pin.set_low().unwrap();

        defmt::trace!("wait for rising edge (on low)");
        input_pin.wait_for_rising_edge().await.unwrap();
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
    hal::binary_info::rp_program_name!(c"rp-hal-async-wait-for-rising-edge"),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"Test for embedded-hal-async / wait for rising edge"),
    hal::binary_info::rp_program_url!(c"private"),
    hal::binary_info::rp_program_build_attribute!(),
];
