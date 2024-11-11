use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use embedded_hal::digital::ErrorType;

use crate::WakerRegister;

use super::{hal, GpioInputPin};

pub(crate) struct WaitForAnyEdge<'a, I, P>
where
    I: hal::gpio::PinId + WakerRegister,
    P: hal::gpio::PullType,
{
    pub(crate) pin: &'a mut GpioInputPin<I, P>,
    pub(crate) polled: bool,
    pub(crate) done: bool,
}

impl<'a, I, P> Unpin for WaitForAnyEdge<'a, I, P>
where
    I: hal::gpio::PinId + WakerRegister,
    P: hal::gpio::PullType,
{
}

impl<'a, I, P> Future for WaitForAnyEdge<'a, I, P>
where
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

                this.pin
                    .set_interrupt_enabled(hal::gpio::Interrupt::EdgeLow, false);
                this.pin
                    .set_interrupt_enabled(hal::gpio::Interrupt::EdgeHigh, false);
            }
            Poll::Ready(Ok(()))
        } else {
            this.polled = true;

            I::register_waker(ctx.waker());

            this.pin
                .set_interrupt_enabled(hal::gpio::Interrupt::EdgeLow, true);
            this.pin
                .set_interrupt_enabled(hal::gpio::Interrupt::EdgeHigh, true);

            Poll::Pending
        }
    }
}

impl<'a, I, P> Drop for WaitForAnyEdge<'a, I, P>
where
    I: hal::gpio::PinId + WakerRegister,
    P: hal::gpio::PullType,
{
    fn drop(&mut self) {
        if self.polled && !self.done {
            self.pin
                .set_interrupt_enabled(hal::gpio::Interrupt::EdgeLow, false);
            self.pin
                .set_interrupt_enabled(hal::gpio::Interrupt::EdgeHigh, false);
        }
    }
}
