// SPDX-License-Identifier: BSD-3-Clause

use core::arch::asm;
use std::panic;

use tracing::error;

#[cfg(feature = "stack-unwinding")]
use crate::debug::{info, trace};

// Panic hook used when we panic prior to larger system initialization
// NOTE(aki): This assumes we are in UEFI text mode
pub fn pre_init_panic(info: &panic::PanicHookInfo<'_>) -> ! {
    // The Rust std crate w/ UEFI should have *very basic* output

    let panic_log = info.location().unwrap();
    let panic_msg = info.payload_as_str().unwrap_or("<No Message>");

    print!("Pre-init panic!\r\n");
    print!("{panic_log}: {panic_msg}\r\n");

    // If we are in debug mode, assume we have access to the QEMU debug serial port
    // Use it to emit a desperate gasp to try to let people know whats going on
    if cfg!(debug_assertions) {
        use core::fmt::Write;
        let mut dbgcon = crate::log::QEMUDebugcon::default();
        let _ = writeln!(dbgcon, "Pre-init panic!");
        let _ = writeln!(dbgcon, "{panic_log}: {panic_msg}");
    }

    loop {
        unsafe {
            asm!("hlt", options(nomem, nostack));
        }
    }
}

// Panic hook for when we're mostly set up.
// BUG(aki):
// There may be a case where we panic *inside* the logging methods
// or the framebuffer so this will cause us to not work as expected
pub fn post_init_panic(info: &panic::PanicHookInfo<'_>) -> ! {
    error!("SYSTEM PANIC");
    let panic_log = info.location().unwrap();
    let panic_msg = info.payload_as_str().unwrap_or("<No Message>");

    error!("{}: {}", panic_log, panic_msg);

    if cfg!(feature = "stack-unwinding") {
        if info::has_unwind_table() {
            // Capture a stack trace from here
            // TODO(aki): get unwinding working
            let _bt = trace::Trace::new();
        } else {
            error!("No unwind table present, unable to unwind stack!");
        }
    } else {
        error!("Stack unwinding not available!");
    }

    loop {
        unsafe {
            asm!("hlt", options(nomem, nostack));
        }
    }
}
