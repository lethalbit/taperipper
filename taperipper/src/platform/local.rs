// SPDX-License-Identifier: BSD-3-Clause
// Implementation inspired by mycelium (https://github.com/hawkw/mycelium)
// by Eliza Weisman.
//
//

use core::{
    any,
    arch::asm,
    fmt,
    marker::PhantomPinned,
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr,
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
};

use maitake_sync::spin::Lazy;
use tracing::{trace, warn};

use crate::platform::msr::GS_BASE;

// TODO(aki): Make `CoreLocals` generic so we can use `GS` and `FS` segmented local storage

// CoreLocals use the `GS` base segment
#[repr(C)]
pub struct CoreLocals {
    _self: *const Self,
    _key: usize,
    _pin: PhantomPinned,
    locals: [AtomicPtr<()>; Self::MAX_LOCALS],
}

impl CoreLocals {
    const LOCALS_KEY: usize = 0x424947424F4F4253;
    const MAX_LOCALS: usize = 64;

    // Check to see if locals for this core are initialized
    fn is_initialized() -> bool {
        // If the GS base is not set, then we can assume we've not been initialized
        if GS_BASE.read() == 0 {
            return false;
        }

        // If `GS` *is* set, then we need to make sure the magic value is set
        let key: usize;
        unsafe {
            // NOTE(aki): This is brittle, it's assuming that `_KEY` is at offset 0x08
            asm!("movq %gs:0x08, {}", out(reg) key, options(att_syntax));
        }

        Self::LOCALS_KEY == key
    }

    // Create a new CoreLocals object full of null pointers
    const fn new() -> Self {
        const LOCAL_SLOT_INIT: AtomicPtr<()> = AtomicPtr::new(ptr::null_mut());
        Self {
            _self: ptr::null(),
            _key: Self::LOCALS_KEY,
            _pin: PhantomPinned,
            locals: [LOCAL_SLOT_INIT; Self::MAX_LOCALS],
        }
    }

    // TODO(aki): Make this return a Result<> so we blow up if we tri to clobber
    #[track_caller]
    pub fn init() {
        if Self::is_initialized() {
            warn!("Core locals already initialized on this core!");
            return;
        }

        // Initialize a new CoreLocals
        let ptr = Box::into_raw(Box::new(Self::new()));
        unsafe {
            // Stuff the reference to the Locals into itself
            (*ptr)._self = ptr as *const _;
            // Write the MSR to set the GS segment to be based on that address
            GS_BASE.write(ptr as u64);
        }
    }

    pub fn try_current() -> Option<Pin<&'static Self>> {
        // Check to make sure we're initialized
        if !Self::is_initialized() {
            trace!("Core locals not initialized!");
            return None;
        }

        // If so, pull out the base address for the structure and stamp out things
        unsafe {
            let ptr: *const Self;
            asm!("movq %gs:0x00, {}", out(reg) ptr, options(att_syntax));
            Some(Pin::new_unchecked(&*ptr))
        }
    }

    #[track_caller]
    pub fn current() -> Pin<&'static Self> {
        Self::try_current()
            .expect("CoreLocals::current() was called before CoreLocals::init() on this core!")
    }

    // NOTE:(aki): This will *initialize* the value if it hasn't been
    pub fn with<T, U>(&self, key: &CoreLocal<T>, func: impl FnOnce(&T) -> U) -> U {
        // Get the index of the local in our local table
        let slot_idx = *key.slot;

        // Get the pointer to the local if the index is within the table
        let slot = match self.locals.get(slot_idx) {
            Some(slot) => slot,
            None => panic!(
                "Index of CoreLocal out of bounds. ({slot_idx} < {})",
                Self::MAX_LOCALS
            ),
        };

        // Get the pointer, and
        let mut ptr = slot.load(Ordering::Acquire);

        // If the slot pointer is null, then
        if ptr.is_null() {
            // Allocate the data and get a type-erased pointer from it
            let data = Box::new((key.init)());
            let dptr = Box::into_raw(data) as *mut ();

            // Stuff the new pointer into the slot.
            let _ = slot
                .compare_exchange(ptr, dptr, Ordering::AcqRel, Ordering::Acquire)
                .expect("Unable to initialize slot pointer with new data from CoreLocal!");

            ptr = dptr;
        }

        // Finally, once we have the data (or it's been initialized) then manifest the object can invoke the func
        let data = unsafe { &*(ptr as *const T) };
        func(data)
    }
}

// TODO(aki): Deref/DerefMut traits?
pub struct CoreLocal<T> {
    slot: Lazy<usize>,
    init: fn() -> T,
}

impl<T: 'static> CoreLocal<T> {
    // Get the next slot
    fn next_slot() -> usize {
        static NEXT_SLOT: AtomicUsize = AtomicUsize::new(0);
        let slot = NEXT_SLOT.fetch_add(1, Ordering::Relaxed);

        // If this local would fall off the end, bail
        assert!(
            slot < CoreLocals::MAX_LOCALS,
            "Next CoreLocal would overflow available {} core local slots!",
            CoreLocals::MAX_LOCALS
        );

        slot
    }

    #[must_use]
    #[track_caller]
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            slot: Lazy::new(Self::next_slot),
            init,
        }
    }

    #[track_caller]
    pub fn with<U>(&self, func: impl FnOnce(&T) -> U) -> U {
        CoreLocals::current().with(self, func)
    }
}

impl<T> fmt::Debug for CoreLocal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CoreLocal<{}>({})",
            any::type_name::<T>(),
            self.slot.get()
        )
    }
}
