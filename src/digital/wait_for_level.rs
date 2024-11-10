use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};

use embedded_hal::digital::{ErrorType, InputPin};

use crate::WakerRegister;

use super::{hal, GpioInputPin};

/// Polymorphic wait for a pin level
pub(crate) trait WaitForLevelInfo<PIN>
where
    PIN: ErrorType,
{
    const INTERRUPT: hal::gpio::Interrupt;

    fn is_level(pin: &mut PIN) -> Result<bool, <PIN as ErrorType>::Error>;
}

pub(crate) struct HighLevelInfo;
impl<I: hal::gpio::PinId + WakerRegister, P: hal::gpio::PullType>
    WaitForLevelInfo<GpioInputPin<I, P>> for HighLevelInfo
{
    const INTERRUPT: hal::gpio::Interrupt = hal::gpio::Interrupt::LevelHigh;

    fn is_level(
        pin: &mut GpioInputPin<I, P>,
    ) -> Result<bool, <GpioInputPin<I, P> as ErrorType>::Error> {
        pin.is_high()
    }
}

pub(crate) struct LowLevelInfo;
impl<I: hal::gpio::PinId + WakerRegister, P: hal::gpio::PullType>
    WaitForLevelInfo<GpioInputPin<I, P>> for LowLevelInfo
{
    const INTERRUPT: hal::gpio::Interrupt = hal::gpio::Interrupt::LevelLow;

    fn is_level(
        pin: &mut GpioInputPin<I, P>,
    ) -> Result<bool, <GpioInputPin<I, P> as ErrorType>::Error> {
        pin.is_low()
    }
}

pub(crate) struct WaitForLevel<'a, INFO, I, P>
where
    INFO: WaitForLevelInfo<GpioInputPin<I, P>>,
    I: hal::gpio::PinId + WakerRegister,
    P: hal::gpio::PullType,
{
    pub(crate) pin: &'a mut GpioInputPin<I, P>,
    pub(crate) polled: bool,
    pub(crate) done: bool,
    pub(crate) _info: PhantomData<INFO>,
}

impl<'a, INFO, I, P> Unpin for WaitForLevel<'a, INFO, I, P>
where
    INFO: WaitForLevelInfo<GpioInputPin<I, P>>,
    I: hal::gpio::PinId + WakerRegister,
    P: hal::gpio::PullType,
{
}

impl<'a, INFO, I, P> Future for WaitForLevel<'a, INFO, I, P>
where
    INFO: WaitForLevelInfo<GpioInputPin<I, P>>,
    I: hal::gpio::PinId + WakerRegister,
    P: hal::gpio::PullType,
{
    type Output = Result<(), <GpioInputPin<I, P> as ErrorType>::Error>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.polled {
            if this.done {
                defmt::error!("poll invoked after ready");
            } else {
                this.done = true;
                this.pin.set_interrupt_enabled(INFO::INTERRUPT, false);
            }
            Poll::Ready(Ok(()))
        } else {
            this.polled = true;

            let level = match INFO::is_level(this.pin) {
                Err(e) => {
                    this.done = true;

                    return Poll::Ready(Err(e));
                }
                Ok(level) => level,
            };

            if level {
                this.done = true;

                Poll::Ready(Ok(()))
            } else {
                I::register_waker(ctx.waker());

                this.pin.set_interrupt_enabled(INFO::INTERRUPT, true);

                Poll::Pending
            }
        }
    }
}

impl<'a, INFO, I, P> Drop for WaitForLevel<'a, INFO, I, P>
where
    INFO: WaitForLevelInfo<GpioInputPin<I, P>>,
    I: hal::gpio::PinId + WakerRegister,
    P: hal::gpio::PullType,
{
    fn drop(&mut self) {
        if self.polled && !self.done {
            self.pin.set_interrupt_enabled(INFO::INTERRUPT, false);
        }
    }
}
