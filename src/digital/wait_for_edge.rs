use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};

use embedded_hal::digital::ErrorType;

use crate::WakerRegister;

use super::{hal, GpioInputPin};

/// Polymorphic wait for an edge
pub(crate) trait WaitForEdgeInfo {
    const INTERRUPT: hal::gpio::Interrupt;
}

pub(crate) struct EdgeHighInfo;
impl WaitForEdgeInfo for EdgeHighInfo {
    const INTERRUPT: hal::gpio::Interrupt = hal::gpio::Interrupt::EdgeHigh;
}

pub(crate) struct EdgeLowInfo;
impl WaitForEdgeInfo for EdgeLowInfo {
    const INTERRUPT: hal::gpio::Interrupt = hal::gpio::Interrupt::EdgeLow;
}

pub(crate) struct WaitForEdge<'a, INFO, I, P>
where
    INFO: WaitForEdgeInfo,
    I: hal::gpio::PinId + WakerRegister,
    P: hal::gpio::PullType,
{
    pub(crate) pin: &'a mut GpioInputPin<I, P>,
    pub(crate) polled: bool,
    pub(crate) done: bool,
    pub(crate) _info: PhantomData<INFO>,
}

impl<'a, INFO, I, P> Unpin for WaitForEdge<'a, INFO, I, P>
where
    INFO: WaitForEdgeInfo,
    I: hal::gpio::PinId + WakerRegister,
    P: hal::gpio::PullType,
{
}

impl<'a, INFO, I, P> Future for WaitForEdge<'a, INFO, I, P>
where
    INFO: WaitForEdgeInfo,
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

            I::register_waker(ctx.waker());

            this.pin.set_interrupt_enabled(INFO::INTERRUPT, true);

            Poll::Pending
        }
    }
}

impl<'a, INFO, I, P> Drop for WaitForEdge<'a, INFO, I, P>
where
    INFO: WaitForEdgeInfo,
    I: hal::gpio::PinId + WakerRegister,
    P: hal::gpio::PullType,
{
    fn drop(&mut self) {
        if self.polled && !self.done {
            self.pin.set_interrupt_enabled(INFO::INTERRUPT, false);
        }
    }
}
