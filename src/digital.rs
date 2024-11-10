mod wait_for_any_edge;
mod wait_for_edge;
mod wait_for_level;

use core::marker::PhantomData;

use embassy_sync::waitqueue::AtomicWaker;

#[cfg(target_arch = "riscv32")]
compile_error!("TODO: riscv32");

#[cfg(feature = "rp235x")]
use rp235x_hal as hal;

#[cfg(feature = "rp2040")]
use rp2040_hal as hal;

use hal::pac::interrupt;

use embedded_hal::digital::{ErrorType, InputPin};
use embedded_hal_async::digital::Wait;

use crate::{get_current_core, write_bitmask_clear, IntoAsync, WakerRegister, NUM_CORES};

type GpioInputPin<I, P> = hal::gpio::Pin<I, hal::gpio::FunctionSio<hal::gpio::SioInput>, P>;

pub struct AsyncInputPin<I: hal::gpio::PinId, P: hal::gpio::PullType> {
    pin: GpioInputPin<I, P>,
}

impl<I: hal::gpio::PinId, P: hal::gpio::PullType> ErrorType for AsyncInputPin<I, P> {
    type Error = <GpioInputPin<I, P> as ErrorType>::Error;
}

impl<I: hal::gpio::PinId, P: hal::gpio::PullType> InputPin for AsyncInputPin<I, P> {
    fn is_high(&mut self) -> Result<bool, <Self as ErrorType>::Error> {
        self.pin.is_high()
    }

    fn is_low(&mut self) -> Result<bool, <Self as ErrorType>::Error> {
        self.pin.is_low()
    }
}

impl<I: hal::gpio::PinId, P: hal::gpio::PullType> AsyncInputPin<I, P> {
    fn new(pin: GpioInputPin<I, P>) -> Self {
        Self { pin }
    }
}

impl<I: hal::gpio::PinId + WakerRegister, P: hal::gpio::PullType> Wait for AsyncInputPin<I, P> {
    async fn wait_for_high(&mut self) -> Result<(), <Self as ErrorType>::Error> {
        wait_for_level::WaitForLevel {
            pin: &mut self.pin,
            polled: false,
            done: false,
            _info: PhantomData::<wait_for_level::HighLevelInfo>,
        }
        .await
    }

    async fn wait_for_low(&mut self) -> Result<(), <Self as ErrorType>::Error> {
        wait_for_level::WaitForLevel {
            pin: &mut self.pin,
            polled: false,
            done: false,
            _info: PhantomData::<wait_for_level::LowLevelInfo>,
        }
        .await
    }

    async fn wait_for_rising_edge(&mut self) -> Result<(), <Self as ErrorType>::Error> {
        wait_for_edge::WaitForEdge {
            pin: &mut self.pin,
            polled: false,
            done: false,
            _info: PhantomData::<wait_for_edge::EdgeHighInfo>,
        }
        .await
    }

    async fn wait_for_falling_edge(&mut self) -> Result<(), <Self as ErrorType>::Error> {
        wait_for_edge::WaitForEdge {
            pin: &mut self.pin,
            polled: false,
            done: false,
            _info: PhantomData::<wait_for_edge::EdgeLowInfo>,
        }
        .await
    }

    async fn wait_for_any_edge(&mut self) -> Result<(), <Self as ErrorType>::Error> {
        wait_for_any_edge::WaitForAnyEdge {
            pin: &mut self.pin,
            polled: false,
            done: false,
        }
        .await
    }
}

impl<I: hal::gpio::PinId, P: hal::gpio::PullType> IntoAsync
    for hal::gpio::Pin<I, hal::gpio::FunctionSio<hal::gpio::SioInput>, P>
{
    type Target = AsyncInputPin<I, P>;

    fn into_async(self) -> Self::Target {
        AsyncInputPin::new(self)
    }
}

const NUM_PINS: usize = 48;

static WAKERS_BANK0: [[AtomicWaker; NUM_PINS]; NUM_CORES] =
    [const { [const { AtomicWaker::new() }; NUM_PINS] }; NUM_CORES];

pub(crate) unsafe fn init() {
    #[cfg(target_arch = "arm")]
    {
        cortex_m::peripheral::NVIC::unmask(hal::pac::interrupt::IO_IRQ_BANK0);
    }

    #[cfg(target_arch = "riscv32")]
    {
        todo!();
    }
}

#[cfg_attr(target_arch = "arm", interrupt)]
fn IO_IRQ_BANK0() {
    let core = get_current_core();

    let wakers = &WAKERS_BANK0[core];

    let bank0 = unsafe { hal::pac::IO_BANK0::steal() };
    if core == 0 {
        for (i, ints) in bank0.proc0_ints_iter().enumerate() {
            for k in 0..8 {
                let event = (ints.read().bits() >> (k * 4)) & 0xf;
                if event != 0 {
                    unsafe { write_bitmask_clear(bank0.proc0_inte(i).as_ptr(), 0xf << (k * 4)) };
                    unsafe { write_bitmask_clear(bank0.intr(i).as_ptr(), 0xf << (k * 4)) };
                    wakers[i * 8 + k].wake();
                }
            }
        }
    } else if core == 1 {
        for (i, ints) in bank0.proc1_ints_iter().enumerate() {
            for k in 0..8 {
                let event = (ints.read().bits() >> (k * 4)) & 0xf;
                if event != 0 {
                    unsafe { write_bitmask_clear(bank0.proc1_inte(i).as_ptr(), 0xf << (k * 4)) };
                    unsafe { write_bitmask_clear(bank0.intr(i).as_ptr(), 0xf << (k * 4)) };
                    wakers[i * 8 + k].wake();
                }
            }
        }
    } else {
        defmt::error!("invalid core {}", core);
    }
}

mod inner {
    use core::task::Waker;

    use crate::{get_current_core, WakerRegister};

    use super::{hal, WAKERS_BANK0};

    macro_rules! pins_impl {
        ($($gpio:ident => $e:expr),+) => {
            $(
                impl WakerRegister for hal::gpio::bank0::$gpio {
                    fn register_waker(waker: &Waker) {
                        WAKERS_BANK0[get_current_core()][$e].register(waker);
                    }
                }
            )+
        }
    }

    pins_impl! {
        Gpio0 => 0,
        Gpio1 => 1,
        Gpio26 => 26
    }
}
