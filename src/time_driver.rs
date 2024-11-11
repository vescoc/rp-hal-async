use core::cell::Cell;
use core::sync::atomic::Ordering;

use portable_atomic::AtomicU8;

#[cfg(feature = "rp235x")]
use rp235x_hal as hal;

#[cfg(feature = "rp235x")]
use hal::pac::TIMER1 as TIMER;

#[cfg(feature = "rp2040")]
use rp2040_hal as hal;

#[cfg(feature = "rp2040")]
use hal::pac::TIMER;

use hal::timer::Alarm;

use critical_section::Mutex;

use embassy_time_driver::{AlarmHandle, Driver};

use hal::pac::interrupt;

use crate::Timer;

#[allow(clippy::type_complexity)]
struct AlarmState {
    timestamp: Cell<u64>,
    callback: Cell<Option<(fn(*mut ()), *mut ())>>,
}
unsafe impl Send for AlarmState {}

const ALARM_COUNT: usize = 4;

struct TimerDriver {
    alarms: Mutex<[AlarmState; ALARM_COUNT]>,
    next_alarm: AtomicU8,
}

embassy_time_driver::time_driver_impl!(
    static DRIVER: TimerDriver = TimerDriver {
        alarms: Mutex::new([const { AlarmState { timestamp: Cell::new(u64::MAX), callback: Cell::new(None) } }; ALARM_COUNT]),
        next_alarm: AtomicU8::new(0),
    }
);

impl Driver for TimerDriver {
    fn now(&self) -> u64 {
        let timer = unsafe { TIMER::steal() };
        let mut high = timer.timerawh().read().bits();
        loop {
            let low = timer.timerawl().read().bits();
            let high2 = timer.timerawh().read().bits();
            if high == high2 {
                return u64::from(high) << 32 | u64::from(low);
            }
            high = high2;
        }
    }

    unsafe fn allocate_alarm(&self) -> Option<AlarmHandle> {
        let id = self
            .next_alarm
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |x| {
                if x < ALARM_COUNT as u8 {
                    Some(x + 1)
                } else {
                    None
                }
            });

        match id {
            Ok(id) => Some(AlarmHandle::new(id)),
            Err(_) => {
                defmt::warn!("cannot allocate alarm");
                None
            }
        }
    }

    fn set_alarm_callback(&self, alarm: AlarmHandle, callback: fn(*mut ()), context: *mut ()) {
        let n = alarm.id() as usize;
        critical_section::with(|cs| {
            let alarm = &self.alarms.borrow(cs)[n];
            alarm.callback.set(Some((callback, context)));
        });
    }

    fn set_alarm(&self, alarm: AlarmHandle, timestamp: u64) -> bool {
        let n = alarm.id() as usize;
        critical_section::with(|cs| {
            let alarm = &self.alarms.borrow(cs)[n];
            alarm.timestamp.set(timestamp);

            let timer = unsafe { TIMER::steal() };
            match n {
                0 => timer
                    .alarm0()
                    .write(|w| unsafe { w.bits(timestamp as u32) }),
                1 => timer
                    .alarm1()
                    .write(|w| unsafe { w.bits(timestamp as u32) }),
                2 => timer
                    .alarm2()
                    .write(|w| unsafe { w.bits(timestamp as u32) }),
                3 => timer
                    .alarm3()
                    .write(|w| unsafe { w.bits(timestamp as u32) }),
                _ => unreachable!(),
            }

            let now = self.now();
            if timestamp <= now {
                unsafe {
                    timer.armed().write_with_zero(|w| w.bits(1 << n));
                }

                alarm.timestamp.set(u64::MAX);

                false
            } else {
                true
            }
        })
    }
}

impl TimerDriver {
    fn check_alarm(&self, n: usize) {
        critical_section::with(|cs| {
            let timer = unsafe { TIMER::steal() };

            let alarm = &self.alarms.borrow(cs)[n];
            let timestamp = alarm.timestamp.get();
            if timestamp <= self.now() {
                unsafe {
                    timer.armed().write_with_zero(|w| w.bits(1 << n));
                }

                alarm.timestamp.set(u64::MAX);

                if let Some((f, ctx)) = alarm.callback.get() {
                    f(ctx);
                }
            }

            unsafe {
                timer.intr().write_with_zero(|w| w.bits(1 << n));
            }
        });
    }
}

pub(crate) unsafe fn init(mut timer: Timer) {
    #[cfg(all(target_arch = "arm", feature = "rp235x"))]
    {
        cortex_m::peripheral::NVIC::unmask(interrupt::TIMER1_IRQ_0);
        cortex_m::peripheral::NVIC::unmask(interrupt::TIMER1_IRQ_1);
        cortex_m::peripheral::NVIC::unmask(interrupt::TIMER1_IRQ_2);
        cortex_m::peripheral::NVIC::unmask(interrupt::TIMER1_IRQ_3);
    }

    #[cfg(all(target_arch = "arm", feature = "rp2040"))]
    {
        cortex_m::peripheral::NVIC::unmask(interrupt::TIMER_IRQ_0);
        cortex_m::peripheral::NVIC::unmask(interrupt::TIMER_IRQ_1);
        cortex_m::peripheral::NVIC::unmask(interrupt::TIMER_IRQ_2);
        cortex_m::peripheral::NVIC::unmask(interrupt::TIMER_IRQ_3);
    }

    #[cfg(target_arch = "riscv32")]
    {
        todo!();
    }

    let mut alarm_0 = timer.alarm_0().unwrap();
    alarm_0.enable_interrupt();
    core::mem::forget(alarm_0);

    let mut alarm_1 = timer.alarm_1().unwrap();
    alarm_1.enable_interrupt();
    core::mem::forget(alarm_1);

    let mut alarm_2 = timer.alarm_2().unwrap();
    alarm_2.enable_interrupt();
    core::mem::forget(alarm_2);

    let mut alarm_3 = timer.alarm_3().unwrap();
    alarm_3.enable_interrupt();
    core::mem::forget(alarm_3);
}

#[cfg(feature = "rp235x")]
mod inner {
    use super::{hal::pac::interrupt, DRIVER};

    #[cfg_attr(target_arch = "arm", interrupt)]
    #[cfg_attr(
        target_arch = "riscv32",
        riscv_rt::external_interrupt(interrupt::TIMER1_IRQ_0)
    )]
    fn TIMER1_IRQ_0() {
        DRIVER.check_alarm(0);
    }

    #[cfg_attr(target_arch = "arm", interrupt)]
    #[cfg_attr(
        target_arch = "riscv32",
        riscv_rt::external_interrupt(interrupt::TIMER1_IRQ_1)
    )]
    fn TIMER1_IRQ_1() {
        DRIVER.check_alarm(1);
    }

    #[cfg_attr(target_arch = "arm", interrupt)]
    #[cfg_attr(
        target_arch = "riscv32",
        riscv_rt::external_interrupt(interrupt::TIMER1_IRQ_2)
    )]
    fn TIMER1_IRQ_2() {
        DRIVER.check_alarm(2);
    }

    #[cfg_attr(target_arch = "arm", interrupt)]
    #[cfg_attr(
        target_arch = "riscv32",
        riscv_rt::external_interrupt(interrupt::TIMER1_IRQ_3)
    )]
    fn TIMER1_IRQ_3() {
        DRIVER.check_alarm(3);
    }
}

#[cfg(feature = "rp2040")]
mod inner {
    use super::{hal::pac::interrupt, DRIVER};

    #[cfg_attr(target_arch = "arm", interrupt)]
    #[cfg_attr(
        target_arch = "riscv32",
        riscv_rt::external_interrupt(interrupt::TIMER_IRQ_0)
    )]
    fn TIMER_IRQ_0() {
        DRIVER.check_alarm(0);
    }

    #[cfg_attr(target_arch = "arm", interrupt)]
    #[cfg_attr(
        target_arch = "riscv32",
        riscv_rt::external_interrupt(interrupt::TIMER_IRQ_1)
    )]
    fn TIMER_IRQ_1() {
        DRIVER.check_alarm(1);
    }

    #[cfg_attr(target_arch = "arm", interrupt)]
    #[cfg_attr(
        target_arch = "riscv32",
        riscv_rt::external_interrupt(interrupt::TIMER_IRQ_2)
    )]
    fn TIMER_IRQ_2() {
        DRIVER.check_alarm(2);
    }

    #[cfg_attr(target_arch = "arm", interrupt)]
    #[cfg_attr(
        target_arch = "riscv32",
        riscv_rt::external_interrupt(interrupt::TIMER_IRQ_3)
    )]
    fn TIMER_IRQ_3() {
        DRIVER.check_alarm(3);
    }
}
