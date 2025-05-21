// SPDX-License-Identifier: BSD-3-Clause

#![feature(uefi_std, panic_payload_as_str, panic_can_unwind)]

use core::{arch::asm, time};
use std::{
    panic,
    sync::{Arc, RwLock},
};

use tracing::{self, debug, error, info, trace, warn};
use uefi::{boot, system};

#[cfg(feature = "stack-unwinding")]
mod debug;
mod display;
mod log;
mod uefi_sys;

#[cfg(feature = "stack-unwinding")]
use crate::debug::{info, trace};
use crate::{
    display::framebuffer::Framebuffer,
    log::{ConsoleSubscriber, GOPConsole, QEMUDebugcon, TXTConsole, writer},
};

use maitake::scheduler::{self, StaticScheduler};

static MAITAKE_SCHED: StaticScheduler = scheduler::new_static!();

fn setup_logging(fb: &Arc<RwLock<Framebuffer>>, level: tracing::Level) {
    // NOTE(aki): This is probably less than ideal, but *shrug*
    #[cfg(debug_assertions)]
    type DebugCon = QEMUDebugcon;
    #[cfg(not(debug_assertions))]
    type DebugCon = writer::NoOutput;

    type GopCon = writer::WithMaxLevel<GOPConsole>;
    type TxtCon = writer::WithMaxLevel<TXTConsole>;

    // If we have a valid Framebuffer, then we can assume GOP initialized, otherwise fall back to text
    if fb.read().unwrap().is_valid() {
        fb.write().unwrap().clear_screen();

        let gop_cons = writer::WithMaxLevel::new(GOPConsole::from_framebuffer(fb.clone()), level);

        let subscriber = ConsoleSubscriber::<GopCon, DebugCon>::primary_only(gop_cons)
            .with_secondary(DebugCon::default());
        if let Err(err) = tracing::subscriber::set_global_default(subscriber) {
            panic!("Unable to setup global trace handler: {err:?}");
        }
    } else {
        let txt_cons = writer::WithMaxLevel::new(TXTConsole::default(), level);

        let subscriber = ConsoleSubscriber::<TxtCon, DebugCon>::primary_only(txt_cons)
            .with_secondary(DebugCon::default());
        if let Err(err) = tracing::subscriber::set_global_default(subscriber) {
            panic!("Unable to setup global trace handler: {err:?}");
        }

        crate::uefi_sys::set_best_stdout_mode();

        warn!("Unable to initialize Graphics Console, falling back to Text");
    }
}

fn main() {
    // Setup the UEFI crate
    crate::uefi_sys::init_uefi();
    // Hook the defaualt std panic handler
    panic::set_hook(Box::new(|pi| panic(pi)));

    let ext_tables = system::with_config_table(uefi_sys::ExtraTables::new);

    // Initialize a Framebuffer, it *might* be empty if our GOP initialization fails
    let fb =
        if let Ok(gop) = uefi_sys::init_graphics(Framebuffer::MAX_WIDTH, Framebuffer::MAX_HEIGHT) {
            Arc::new(RwLock::new(Framebuffer::from_uefi(gop)))
        } else {
            Arc::new(RwLock::new(Framebuffer::default()))
        };

    setup_logging(&fb, tracing::Level::DEBUG);

    if cfg!(feature = "stack-unwinding") {
        if let Err(err) = info::load_unwind_table() {
            warn!(
                "Unable to load unwind information, stack traces on panic will not be available!"
            );
            warn!("Error: {err:?}");
        }
    }

    debug!("UEFI Version {}", system::uefi_revision());
    debug!("Firmware Vendor  {}", system::firmware_vendor());
    debug!("Firmware Version {}", system::firmware_revision());
    debug!("ACPI Address:   {:#018x}", ext_tables.acpi as usize);
    debug!("SMBIOS Address: {:#018x}", ext_tables.smbios as usize);

    if fb.read().unwrap().is_valid() {
        let fb_size_pixels = (fb.read().unwrap().width(), fb.read().unwrap().height());
        let fb_size_chars = (
            fb.read().unwrap().width_chars(),
            fb.read().unwrap().height_chars(),
        );
        debug!(
            "Framebuffer Resolution: {}x{}",
            fb_size_pixels.0, fb_size_pixels.1
        );
        debug!("Console Size:  {}x{}", fb_size_chars.0, fb_size_chars.1);
    }

    info!("Taperipper v{}", env!("CARGO_PKG_VERSION"));

    MAITAKE_SCHED.spawn(async {
        debug!("Hello everynyan!");
        panic!("OW");
    });

    // TODO(aki):
    // This is:
    //  1) Kinda silly, need a better way to do it
    //  2) Not SMP-friendly, we need to figure out how to get MP executor stuff working
    //  3) Missing UEFI event triggers for tasks, which entails
    //    a) We need to be able to have a task wait for a specific event
    //    b) Have said event be able to wake the task that wants it
    //    c) Deal with the timer task, so we can have some sort of clock for non-blocking sleeps
    loop {
        let tick = MAITAKE_SCHED.tick();
        if !tick.has_remaining {
            boot::stall(1000);
        }
    }

    crate::uefi_sys::shutdown_now();
}

pub fn panic(info: &panic::PanicHookInfo<'_>) -> ! {
    // BUG(aki): There may be a case where we panic *before* logging is set up!

    error!("SYSTEM PANIC");
    let panic_log = info.location().unwrap();
    let panic_msg = info.payload_as_str().unwrap_or("<No Message>");

    error!("{}: {}", panic_log, panic_msg);

    if cfg!(feature = "stack-unwinding") {
        if info::has_unwind_table() {
            // Capture a stack trace from here
            let bt = trace::Trace::new();
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
