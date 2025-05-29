// SPDX-License-Identifier: BSD-3-Clause

use core::arch;
use std::sync::{
    OnceLock,
    atomic::{AtomicU32, Ordering},
};

use maitake::time::{self, Clock, Duration, Timer};
use tracing::{debug, trace};
use uefi::boot;

use crate::platform;

static MAITAKE_TIMER: OnceLock<Timer> = OnceLock::new();
static RDTSC_SHIFT: AtomicU32 = AtomicU32::new(u32::MAX);

fn _duration_from_rdtsc() -> Duration {
    // Total number of attempts to get RDTSC duration
    const MAX_ATTEMPTS: u8 = 5;
    const ELAPSE_DURATION: Duration = Duration::from_millis(50);

    for attempt in 0..MAX_ATTEMPTS {
        trace!(
            "Trying to derive RDTSC frequency: {}/{}",
            attempt, MAX_ATTEMPTS
        );

        // Get elapsed cycle count over ELAPSE_DURATION
        let tsc_start = unsafe { arch::x86_64::_rdtsc() };
        boot::stall(ELAPSE_DURATION.as_micros() as usize);
        let tsc_end = unsafe { arch::x86_64::_rdtsc() };
        let elapsed = tsc_end - tsc_start;
        trace!("Elapsed cycle count after 50ms: {}", elapsed);

        // Try to derive tick duration
        let mut sft = 0;
        while sft < 64 {
            let elapsed: u32 = (elapsed >> sft).try_into().expect("RDTSC cycle overflow");
            let duration = ELAPSE_DURATION / elapsed;

            if duration.as_nanos() > 0 {
                trace!("RDTSC shift: {}", sft);
                let _ = RDTSC_SHIFT.compare_exchange(
                    u32::MAX,
                    sft,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                );
                return duration;
            } else {
                sft += 1;
            }
        }
        trace!("RDTSC shift exhausted, trying again...");
    }
    unreachable!("Unable to calibrate RDTSC");
}

pub fn new_clock() -> Clock {
    if let Ok(ts_props) = platform::uefi::time::get_timestamp_properties() {
        // We have UEFI `Timestamp` protocol support, use that
        trace!("Using UEFI Timestamp protocol for wall clock");
        Clock::new(
            {
                let tick_ns = 1_000_000_000 / ts_props.frequency;
                Duration::from_nanos(tick_ns)
            },
            platform::uefi::time::get_timestamp,
        )
    } else {
        // We don't support the UEFI `Timestamp` protocol, fall back to `rdtsc`
        trace!("Using x86 RDTSC for wall clock");
        Clock::new(_duration_from_rdtsc(), || {
            let tick = unsafe { arch::x86_64::_rdtsc() };
            let shift = RDTSC_SHIFT.load(Ordering::Relaxed);
            tick >> shift
        })
    }
    .named("timestamp-counter")
}

pub fn init_timer() {
    debug!("Initializing global timer");
    let timer = MAITAKE_TIMER.get_or_init(|| Timer::new(new_clock()));
    // TODO(aki): Do we want to panic here or stuff this into the init call above so it only happens once?
    time::set_global_timer(timer).expect("Global timer initialization called more than once!");
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
