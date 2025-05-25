// SPDX-License-Identifier: BSD-3-Clause

use std::{
    cell::Cell,
    sync::atomic::{AtomicUsize, Ordering},
};

use maitake::{
    scheduler::{Injector, StaticScheduler, Stealer, TaskStub},
    task::JoinHandle,
};

use maitake_sync::spin::InitOnce;

use crate::platform::{local, smp};

pub mod executor;
pub mod panic;
pub mod time;

static CORE_SCHED: local::CoreLocal<Cell<Option<&'static StaticScheduler>>> =
    local::CoreLocal::new(|| Cell::new(None));

static RUNTIME: Runtime = {
    #[allow(clippy::declare_interior_mutable_const)]
    const UNINITIALIZED_SCHEDS: InitOnce<StaticScheduler> = InitOnce::uninitialized();
    Runtime {
        cores: AtomicUsize::new(0),
        schedulers: [UNINITIALIZED_SCHEDS; smp::MAX_CORES],
        sched_inject: {
            static TASK_STUB: TaskStub = TaskStub::new();
            unsafe { Injector::new_with_static_stub(&TASK_STUB) }
        },
    }
};

struct Runtime {
    cores: AtomicUsize,
    schedulers: [InitOnce<StaticScheduler>; smp::MAX_CORES],
    sched_inject: Injector<&'static StaticScheduler>,
}

impl Runtime {
    fn active_cores(&self) -> usize {
        self.cores.load(Ordering::Acquire)
    }

    fn make_scheduler(&self) -> (usize, &StaticScheduler) {
        // Increment the number of active cores
        let next = self.cores.fetch_add(1, Ordering::AcqRel);

        // TODO(aki): This should be a Result<> so we handle next > MAX_CORES gracefully
        assert!(
            next < smp::MAX_CORES,
            "Unable to make new scheduler, out of core slots ({next} > {})",
            smp::MAX_CORES
        );

        // Initialize a scheduler for that core
        let scheduler = self.schedulers[next].init(StaticScheduler::new());

        // Return the number and the scheduler
        (next, scheduler)
    }

    fn seize(&'static self, core: usize) -> Option<Stealer<'static, &'static StaticScheduler>> {
        self.schedulers[core].try_get()?.try_steal().ok()
    }
}

#[inline]
#[track_caller]
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    CORE_SCHED.with(|sched_cell| {
        // If we have a core-local scheduler spawn directly on that
        if let Some(scheduler) = sched_cell.get() {
            scheduler.spawn(future)
        } else {
            // Otherwise stuff it into the main runtime
            RUNTIME.sched_inject.spawn(future)
        }
    })
}

/// Initialize the Async runtime and
/// create an executor for the boot core
pub fn init() -> executor::CoreExecutor {
    // Initialize the timer subsystem
    time::init_timer();
    // Initialize locals for the boot core
    local::CoreLocals::init();
    // Spawn a new core executor
    executor::CoreExecutor::new()
}
