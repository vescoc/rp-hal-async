use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[cfg(target_arch = "riscv32")]
compile_error!("TODO: riscv32");

#[cfg(feature = "rp235x")]
use rp235x_hal as hal;

#[cfg(feature = "rp2040")]
use rp2040_hal as hal;

use embedded_hal_async::delay::DelayNs;

use crate::{WakerRegister, NUM_CORES};

struct AsyncAlarmFut<'a, A: hal::timer::Alarm> {
    alarm: &'a mut A,
    delay: u32,
    polled: bool,
    done: bool,
}

impl<'a, A: hal::timer::Alarm> Unpin for AsyncAlarmFut<'a, A> {}

impl<'a, A: hal::timer::Alarm> Drop for AsyncAlarmFut<'a, A> {
    fn drop(&mut self) {
        if self.polled && !self.done {
            self.alarm.disable_interrupt();
        }
    }
}

impl<'a, A> Future for AsyncAlarmFut<'a, A>
where
    A: hal::timer::Alarm + WakerRegister,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.polled {
            if this.done {
                defmt::error!("poll invoked after ready");
            } else {
                this.done = true;
                this.alarm.disable_interrupt();
            }

            Poll::Ready(())
        } else {
            let duration = fugit::MicrosDurationU32::micros(this.delay);

            this.polled = true;

            A::register_waker(&cx.waker());

            this.alarm.schedule(duration).unwrap();
            this.alarm.enable_interrupt();

            Poll::Pending
        }
    }
}

pub struct AsyncAlarm<A>
where
    A: hal::timer::Alarm,
{
    alarm: A,
}

impl<A: hal::timer::Alarm> AsyncAlarm<A> {
    fn new(alarm: A) -> Self {
        Self { alarm }
    }
}

impl<A> DelayNs for AsyncAlarm<A>
where
    A: hal::timer::Alarm + WakerRegister,
{
    async fn delay_ns(&mut self, delay: u32) {
        let delay = delay.div_ceil(1_000);
        if delay > 0 {
            AsyncAlarmFut {
                alarm: &mut self.alarm,
                delay,
                polled: false,
                done: false,
            }
            .await;
        }
    }
}

pub(crate) unsafe fn init() {
    #[cfg(all(target_arch = "arm", feature = "rp235x"))]
    {
        cortex_m::peripheral::NVIC::unmask(hal::pac::interrupt::TIMER0_IRQ_0);
        cortex_m::peripheral::NVIC::unmask(hal::pac::interrupt::TIMER0_IRQ_1);
        cortex_m::peripheral::NVIC::unmask(hal::pac::interrupt::TIMER0_IRQ_2);
        cortex_m::peripheral::NVIC::unmask(hal::pac::interrupt::TIMER0_IRQ_3);
        cortex_m::peripheral::NVIC::unmask(hal::pac::interrupt::TIMER1_IRQ_0);
        cortex_m::peripheral::NVIC::unmask(hal::pac::interrupt::TIMER1_IRQ_1);
        cortex_m::peripheral::NVIC::unmask(hal::pac::interrupt::TIMER1_IRQ_2);
        cortex_m::peripheral::NVIC::unmask(hal::pac::interrupt::TIMER1_IRQ_3);
    }

    #[cfg(all(target_arch = "arm", feature = "rp2040"))]
    {
        cortex_m::peripheral::NVIC::unmask(hal::pac::interrupt::TIMER_IRQ_0);
        cortex_m::peripheral::NVIC::unmask(hal::pac::interrupt::TIMER_IRQ_1);
        cortex_m::peripheral::NVIC::unmask(hal::pac::interrupt::TIMER_IRQ_2);
        cortex_m::peripheral::NVIC::unmask(hal::pac::interrupt::TIMER_IRQ_3);
    }

    #[cfg(target_arch = "riscv32")]
    {
        todo!();
    }
}

#[cfg(feature = "rp235x")]
mod inner {
    macro_rules! alarm_impl {
        (mod_name: $mod_name:ident, name: $alarm:ident, alarm_name: $alarm_name:ident, device: $device:ident, wakers: $wakers:ident, irq: $irq:ident, timer: $timer:ident, mask: $mask:expr) => {
            mod $mod_name {
                use crate::{get_current_core, write_bitmask_clear};
                use core::task::Waker;

                use crate::delay::{hal, AsyncAlarm, NUM_CORES};

                use embassy_sync::waitqueue::AtomicWaker;

                use hal::pac::interrupt;

                use crate::{IntoAsync, WakerRegister};

                impl IntoAsync for hal::timer::$alarm_name<hal::timer::$device> {
                    type Target = AsyncAlarm<hal::timer::$alarm_name<hal::timer::$device>>;

                    fn into_async(self) -> Self::Target {
                        AsyncAlarm::new(self)
                    }
                }

                static $wakers: [AtomicWaker; NUM_CORES] =
                    [const { AtomicWaker::new() }; NUM_CORES];

                impl WakerRegister for hal::timer::$alarm_name<hal::timer::$device> {
                    fn register_waker(waker: &Waker) {
                        $wakers[get_current_core()].register(waker);
                    }
                }

                #[cfg_attr(target_arch = "arm", interrupt)]
                fn $irq() {
                    $wakers[get_current_core()].wake();

                    unsafe {
                        let timer = hal::pac::$timer::steal();

                        write_bitmask_clear(timer.inte().as_ptr(), $mask);

                        timer
                            .intr()
                            .write_with_zero(|w| w.$alarm().clear_bit_by_one());
                    }
                }
            }
        };
    }

    alarm_impl! {
        mod_name: timer0_alarm0,
        name: alarm_0,
        alarm_name: Alarm0,
        device: CopyableTimer0,
        wakers: TIMER0_IRQ_0_WAKERS,
        irq: TIMER0_IRQ_0,
        timer: TIMER0,
        mask: 0b0001
    }

    alarm_impl! {
        mod_name: timer0_alarm1,
        name: alarm_1,
        alarm_name: Alarm1,
        device: CopyableTimer0,
        wakers: TIMER0_IRQ_1_WAKERS,
        irq: TIMER0_IRQ_1,
        timer: TIMER0,
        mask: 0b0010
    }

    alarm_impl! {
        mod_name: timer0_alarm2,
        name: alarm_2,
        alarm_name: Alarm2,
        device: CopyableTimer0,
        wakers: TIMER0_IRQ_2_WAKERS,
        irq: TIMER0_IRQ_2,
        timer: TIMER0,
        mask: 0b0100
    }

    alarm_impl! {
        mod_name: timer0_alarm3,
        name: alarm_3,
        alarm_name: Alarm3,
        device: CopyableTimer0,
        wakers: TIMER0_IRQ_3_WAKERS,
        irq: TIMER0_IRQ_3,
        timer: TIMER0,
        mask: 0b1000
    }

    alarm_impl! {
        mod_name: timer1_alarm0,
        name: alarm_0,
        alarm_name: Alarm0,
        device: CopyableTimer1,
        wakers: TIMER1_IRQ_0_WAKERS,
        irq: TIMER1_IRQ_0,
        timer: TIMER1,
        mask: 0b0001
    }

    alarm_impl! {
        mod_name: timer1_alarm1,
        name: alarm_1,
        alarm_name: Alarm1,
        device: CopyableTimer1,
        wakers: TIMER1_IRQ_1_WAKERS,
        irq: TIMER1_IRQ_1,
        timer: TIMER1,
        mask: 0b0010
    }

    alarm_impl! {
        mod_name: timer1_alarm2,
        name: alarm_2,
        alarm_name: Alarm2,
        device: CopyableTimer1,
        wakers: TIMER1_IRQ_2_WAKERS,
        irq: TIMER1_IRQ_2,
        timer: TIMER1,
        mask: 0b0100
    }

    alarm_impl! {
        mod_name: timer1_alarm3,
        name: alarm_3,
        alarm_name: Alarm3,
        device: CopyableTimer1,
        wakers: TIMER1_IRQ_3_WAKERS,
        irq: TIMER1_IRQ_3,
        timer: TIMER1,
        mask: 0b1000
    }
}

#[cfg(feature = "rp2040")]
mod inner {
    macro_rules! alarm_impl {
        (mod_name: $mod_name:ident, name: $alarm:ident, alarm_name: $alarm_name:ident, wakers: $wakers:ident, irq: $irq:ident, timer: $timer:ident, mask: $mask:expr) => {
            mod $mod_name {
                use crate::{get_current_core, write_bitmask_clear};
                use core::task::Waker;

                use crate::delay::{hal, AsyncAlarm, NUM_CORES};

                use embassy_sync::waitqueue::AtomicWaker;

                use hal::pac::interrupt;

                use crate::{IntoAsync, WakerRegister};

                impl IntoAsync for hal::timer::$alarm_name {
                    type Target = AsyncAlarm<hal::timer::$alarm_name>;

                    fn into_async(self) -> Self::Target {
                        AsyncAlarm::new(self)
                    }
                }

                static $wakers: [AtomicWaker; NUM_CORES] =
                    [const { AtomicWaker::new() }; NUM_CORES];

                impl WakerRegister for hal::timer::$alarm_name {
                    fn register_waker(waker: &Waker) {
                        $wakers[get_current_core()].register(waker);
                    }
                }

                #[cfg_attr(target_arch = "arm", interrupt)]
                fn $irq() {
                    $wakers[get_current_core()].wake();

                    unsafe {
                        let timer = hal::pac::$timer::steal();

                        write_bitmask_clear(timer.inte().as_ptr(), $mask);

                        timer
                            .intr()
                            .write_with_zero(|w| w.$alarm().clear_bit_by_one());
                    }
                }
            }
        };
    }

    alarm_impl! {
        mod_name: timer_alarm0,
        name: alarm_0,
        alarm_name: Alarm0,
        wakers: TIMER_IRQ_0_WAKERS,
        irq: TIMER_IRQ_0,
        timer: TIMER,
        mask: 0b0001
    }

    alarm_impl! {
        mod_name: timer_alarm1,
        name: alarm_1,
        alarm_name: Alarm1,
        wakers: TIMER_IRQ_1_WAKERS,
        irq: TIMER_IRQ_1,
        timer: TIMER,
        mask: 0b0010
    }

    alarm_impl! {
        mod_name: timer_alarm2,
        name: alarm_2,
        alarm_name: Alarm2,
        wakers: TIMER_IRQ_2_WAKERS,
        irq: TIMER_IRQ_2,
        timer: TIMER,
        mask: 0b0100
    }

    alarm_impl! {
        mod_name: timer_alarm3,
        name: alarm_3,
        alarm_name: Alarm3,
        wakers: TIMER_IRQ_3_WAKERS,
        irq: TIMER_IRQ_3,
        timer: TIMER,
        mask: 0b1000
    }
}
