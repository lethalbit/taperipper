// SPDX-License-Identifier: BSD-3-Clause

use core::{
    arch, cmp,
    sync::atomic::{AtomicBool, Ordering},
};

use maitake::scheduler::StaticScheduler;

use rand::Rng;
use rand_core::SeedableRng;
use rand_xoshiro::Xoshiro256PlusPlus;
use tracing::{debug, error, info, trace};

use crate::runtime::{CORE_SCHED, RUNTIME, time};

pub struct CoreExecutor {
    sched: &'static StaticScheduler,
    core_id: usize,
    running: AtomicBool,
    healthy: AtomicBool,
    rand: Xoshiro256PlusPlus,
}

impl CoreExecutor {
    // XXX(aki):
    // These have been chosen ✨ arbitrarily ✨
    // It would probably be good to do some dynamic adjustments based on things,
    const MAX_STEAL_ATTEMPTS: usize = 8;
    const MAX_TASKS_TO_STEAL: usize = 64;

    #[must_use]
    pub fn new() -> Self {
        let (id, scheduler) = RUNTIME.make_scheduler();

        info!(core = id, "Initialized task executor");

        let mut seed: u64 = 0;
        let _ = unsafe { arch::x86_64::_rdrand64_step(&mut seed) };

        Self {
            sched: scheduler,
            core_id: id,
            running: AtomicBool::new(false),
            healthy: AtomicBool::new(true),
            rand: Xoshiro256PlusPlus::seed_from_u64(seed),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    pub fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Acquire)
    }

    pub fn stop(&self) -> bool {
        info!(core = self.core_id, "Stopping executor");
        let was_running = self
            .running
            .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
            .is_ok();

        trace!(core = self.core_id, "Executor was running? {was_running}");

        was_running
    }

    pub fn run(&mut self) {
        info!(core = self.core_id, "Starting task executor");

        // When the run loop exits, make sure we dump the Scheduler
        struct _SchGuard;
        impl Drop for _SchGuard {
            fn drop(&mut self) {
                CORE_SCHED.with(|sched_cell| sched_cell.set(None));
            }
        }

        // Make sure we're not already running
        if self
            .running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            error!(
                core = self.core_id,
                "Task executor for core already running!"
            );
            return;
        }

        // Set the core-local scheduler to be the one we were assigned
        CORE_SCHED.with(|sched_cell| sched_cell.set(Some(self.sched)));
        let _sched_cleanup = _SchGuard;

        // Run the scheduler
        loop {
            // Keep ticking as long as we have tasks to poke
            if self.tick() {
                continue;
            }

            // If we're out of work, and we're told to shutdown, do so
            if !self.is_running() {
                info!(core = self.core_id, "Shutting down task executor");
                // _SchGaurd drops and cleans up the scheduler here
                return;
            }
        }
    }

    fn tick(&mut self) -> bool {
        // TODO(aki): Deal with per-core interrupts and IO bits

        let tck = self.sched.tick();
        time::timer().turn();

        if tck.has_remaining {
            return true;
        }

        self.seize() > 0
    }

    fn seize(&mut self) -> usize {
        // Try to get a handle on the task stealer from the runtime injector
        if let Ok(stealer) = RUNTIME.sched_inject.try_steal() {
            return stealer.spawn_n(&self.sched, Self::MAX_TASKS_TO_STEAL);
        }

        // Otherwise, we do it the long way
        let mut attempts_remaining = Self::MAX_STEAL_ATTEMPTS;
        while attempts_remaining != 0 {
            let active = RUNTIME.active_cores();

            // We're oh so lonely, being the only running executor,
            // Or we're a ghost, and really shouldn't be here, oops
            if active <= 1 {
                break;
            }

            // Get the core we want to try to steal from
            let index = if active == 2 {
                // If there is only 1 other core, don't bother with the PRNG
                1
            } else {
                // If there are more than 2 cores, get a random one
                self.rand.random_range(0..active)
            };

            if let Some(victim) = RUNTIME.seize(index) {
                // Figure out how many tasks we want to steal, either half the victims tasks
                // or the max number we are allowed to take, whichever is smaller.
                let theft_count =
                    cmp::min(victim.initial_task_count() / 2, Self::MAX_TASKS_TO_STEAL);
                // We have a stealer from the target core
                return victim.spawn_n(&self.sched, theft_count);
            } else {
                // Welp, lets try again!
                attempts_remaining -= 1;
            }
        }

        // If we exhausted our attempts above, try one more time with the runtime injector
        if let Ok(stealer) = RUNTIME.sched_inject.try_steal() {
            return stealer.spawn_n(&self.sched, Self::MAX_TASKS_TO_STEAL);
        } else {
            0
        }
    }
}
