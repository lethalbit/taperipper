// SPDX-License-Identifier: BSD-3-Clause

use core::arch;
use std::sync::OnceLock;

use maitake::time::{self, Clock, Duration, Timer};
use uefi::boot;

use crate::uefi_sys;

static MAITAKE_TIMER: OnceLock<Timer> = OnceLock::new();

pub fn new_clock() -> Clock {
    if let Ok(ts_props) = uefi_sys::get_timestamp_properties() {
        // We have UEFI `Timestamp` protocol support, use that
        Clock::new(
            {
                let tick_ns = 1_000_000_000 / ts_props.frequency;
                Duration::from_nanos(tick_ns)
            },
            uefi_sys::get_timestamp,
        )
    } else {
        // We don't support the UEFI `Timestamp` protocol, fall back to `rdtsc`
        Clock::new(
            {
                let tsc_start = unsafe { arch::x86_64::_rdtsc() };
                boot::stall(1);
                let tsc_end = unsafe { arch::x86_64::_rdtsc() };
                let tick_ns = 1_000_000_000 / (tsc_end - tsc_start);

                Duration::from_nanos(tick_ns)
            },
            || unsafe { arch::x86_64::_rdtsc() },
        )
    }
    .named("timestamp-counter")
}

pub fn init_timer() {
    let timer = MAITAKE_TIMER.get_or_init(|| Timer::new(new_clock()));
    let _ = time::set_global_timer(&timer);
}

pub fn timer() -> &'static Timer {
    MAITAKE_TIMER.get().unwrap()
}

// XXX(aki): Comment here so I don't forget how to use the silly UEFI timers
// static NYA: AtomicU64 = AtomicU64::new(0);
// extern "efiapi" fn tick(event: Event, ctx: Option<NonNull<c_void>>) {
//     NYA.fetch_add(1, Ordering::Acquire);
// }
// let evt = unsafe {
//     boot::create_event(
//         boot::EventType::TIMER | boot::EventType::NOTIFY_SIGNAL,
//         boot::Tpl::NOTIFY,
//         Some(tick),
//         None,
//     )
// }
// .unwrap();
// boot::set_timer(&evt, boot::TimerTrigger::Periodic(1)).unwrap();
