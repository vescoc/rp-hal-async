#![no_std]

use core::task::Waker;

#[cfg(feature = "delay")]
mod delay;

#[cfg(feature = "digital")]
mod digital;

#[cfg(feature = "time_driver")]
mod time_driver;

#[cfg(feature = "rp235x")]
use rp235x_hal as hal;

#[cfg(feature = "rp2040")]
use rp2040_hal as hal;

#[allow(dead_code)]
trait WakerRegister {
    fn register_waker(waker: &Waker);
}

pub trait IntoAsync {
    type Target;

    fn into_async(self) -> Self::Target;
}

#[allow(dead_code)]
pub(crate) unsafe fn write_bitmask_set(register: *mut u32, bits: u32) {
    core::ptr::write_volatile(register.byte_offset(0x2000), bits);
}

#[allow(dead_code)]
pub(crate) unsafe fn write_bitmask_clear(register: *mut u32, bits: u32) {
    core::ptr::write_volatile(register.byte_offset(0x3000), bits);
}

/// # Safety
#[cfg(feature = "time_driver")]
pub unsafe fn init(timer: hal::timer::Timer<hal::timer::CopyableTimer1>) {
    time_driver::init(timer);

    #[cfg(feature = "digital")]
    digital::init();
}

/// # Safety
#[cfg(not(feature = "time_driver"))]
pub unsafe fn init() {
    #[cfg(feature = "delay")]
    delay::init();

    #[cfg(feature = "digital")]
    digital::init();
}

#[allow(dead_code)]
const NUM_CORES: usize = 2;

#[allow(dead_code)]
fn get_current_core() -> usize {
    unsafe { hal::pac::SIO::steal().cpuid().read().bits() as usize }
}
