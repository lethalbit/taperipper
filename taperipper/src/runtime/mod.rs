// SPDX-License-Identifier: BSD-3-Clause

use maitake::{
    scheduler::{self, StaticScheduler},
    task::JoinHandle,
};

use uefi::boot;

pub mod panic;
pub mod smp;
pub mod time;

pub static MAITAKE_SCHED: StaticScheduler = scheduler::new_static!();

#[inline]
#[track_caller]
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    MAITAKE_SCHED.spawn(future)
}

pub fn run_scheduler() -> ! {
    // TODO(aki):
    // This is:
    //  1) Not SMP-friendly, we need to figure out how to get MP executor stuff working
    //  2) Missing UEFI event triggers for tasks, which entails
    //    a) We need to be able to have a task wait for a specific event
    //    b) Have said event be able to wake the task that wants it
    loop {
        let tick = MAITAKE_SCHED.tick();
        let _ = time::timer().turn();
        if !tick.has_remaining {
            boot::stall(1);
        }
    }
}
