// SPDX-License-Identifier: BSD-3-Clause

#![feature(
    uefi_std,
    panic_payload_as_str,
    panic_can_unwind,
    duration_constructors_lite
)]

use maitake::time;
use std::{
    panic,
    str::FromStr,
    sync::{Arc, RwLock},
};
use tracing::{self, debug, error, info, trace, warn};
use uefi::system;

#[cfg(feature = "stack-unwinding")]
mod debug;
mod display;
mod log;
mod platform;
mod runtime;

#[cfg(feature = "stack-unwinding")]
use crate::debug::info;
use crate::{
    display::framebuffer::Framebuffer,
    log::{ConsoleSubscriber, GOPConsole, QEMUDebugcon, TXTConsole, writer},
};

#[cfg(debug_assertions)]
const DEFAULT_LOG_LEVEL: tracing::Level = tracing::Level::DEBUG;
#[cfg(not(debug_assertions))]
const DEFAULT_LOG_LEVEL: tracing::Level = tracing::Level::INFO;

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

        crate::platform::uefi::set_best_stdout_mode();

        warn!("Unable to initialize Graphics Console, falling back to Text");
    }
}

fn main() {
    // Setup the UEFI crate
    crate::platform::uefi::init_uefi();
    // Set up the pre-system initialization hook
    panic::set_hook(Box::new(|panic_info| {
        runtime::panic::pre_init_panic(panic_info)
    }));

    let ext_tables = system::with_config_table(crate::platform::uefi::ExtraTables::new);

    // Initialize a Framebuffer, it *might* be empty if our GOP initialization fails
    let fb = if let Ok(gop) =
        crate::platform::uefi::init_graphics(Framebuffer::MAX_WIDTH, Framebuffer::MAX_HEIGHT)
    {
        Arc::new(RwLock::new(Framebuffer::from_uefi(gop)))
    } else {
        Arc::new(RwLock::new(Framebuffer::default()))
    };

    // Get the Log level from the UEFI vars, or a default level
    let log_level = if let Some(var) = crate::platform::uefi::get_var("TAPERIPPER_LOG_LEVEL") {
        tracing::Level::from_str(str::from_utf8(&var).unwrap_or("Debug"))
            .unwrap_or(DEFAULT_LOG_LEVEL)
    } else {
        DEFAULT_LOG_LEVEL
    };

    setup_logging(&fb, log_level);

    // Now that we have logging and such, we can set the "post init" panic handler
    trace!("Setting post-init panic handler...");
    panic::set_hook(Box::new(|panic_info| {
        runtime::panic::post_init_panic(panic_info)
    }));

    if cfg!(feature = "stack-unwinding") {
        if let Err(err) = info::load_unwind_table() {
            warn!(
                "Unable to load unwind information, stack traces on panic will not be available!"
            );
            warn!("Error: {err:?}");
        }
    }

    debug!("UEFI Version: {}", system::uefi_revision());
    debug!("Firmware Vendor: {}", system::firmware_vendor());
    debug!("Firmware Version: {}", system::firmware_revision());
    debug!("ACPI Address: {:#018x}", ext_tables.acpi as usize);
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
        debug!("Console Size: {}x{}", fb_size_chars.0, fb_size_chars.1);
    }

    info!("Taperipper v{}", env!("CARGO_PKG_VERSION"));

    runtime::time::init_timer();

    runtime::spawn(async {
        loop {
            time::sleep(time::Duration::from_millis(700)).await;
            debug!("Hello everynyan!");
        }
    });

    runtime::spawn(async {
        loop {
            time::sleep(time::Duration::from_millis(750)).await;
            debug!("Meow");
        }
    });

    runtime::spawn(async {
        time::sleep(time::Duration::from_secs(60)).await;
        panic!("AWAWAWAWAWAW");
    });

    runtime::run_scheduler();

    crate::platform::uefi::shutdown_now();
}
