// SPDX-License-Identifier: BSD-3-Clause

#![feature(
    uefi_std,
    panic_payload_as_str,
    panic_can_unwind,
    duration_constructors_lite,
    allocator_api
)]

use maitake::time;
use std::{
    panic,
    str::FromStr,
    sync::{Arc, RwLock},
};
use tracing::{self, Level, debug, error, info, trace, warn};
use tracing_core::LevelFilter;
use tracing_subscriber::{Layer, filter::Targets, layer::SubscriberExt, util::SubscriberInitExt};
use uefi::system;

#[cfg(feature = "stack-unwinding")]
mod debug;
mod display;
mod log;
mod platform;
mod runtime;

#[cfg(feature = "stack-unwinding")]
use crate::debug::info;
use crate::display::framebuffer::Framebuffer;

#[cfg(debug_assertions)]
const DEFAULT_LOG_LEVEL: tracing::Level = tracing::Level::DEBUG;
#[cfg(not(debug_assertions))]
const DEFAULT_LOG_LEVEL: tracing::Level = tracing::Level::INFO;

fn setup_logging(fb: &Arc<RwLock<Framebuffer>>, level: tracing::Level) {
    let fb_valid = fb.read().unwrap().is_valid();

    let filter = Targets::new()
        .with_default(level)
        // Goblin is so damn noisy, turn it off
        .with_target("goblin", LevelFilter::OFF);

    tracing_subscriber::registry()
        .with(fb_valid.then(|| {
            // Our framebuffer is valid, clear the screen then set up the layer
            fb.write().unwrap().clear_screen();
            log::gop_cons::framebuffer_layer(fb.clone()).with_filter(filter.clone())
        }))
        .with((!fb_valid).then(|| {
            // If the GOP Framebuffer is not valid, then fall back to UEFI Text mode
            platform::uefi::output::set_best_stdout_mode();
            log::txt_cons::layer().with_filter(filter)
        }))
        .with(cfg!(debug_assertions).then(|| {
            // If we are in debug mode, assume the QEMU Debug port is there
            log::qemu::layer().with_filter(
                Targets::new()
                    // Emit trace info to the debug console
                    .with_default(Level::TRACE)
                    // Goblin is so damn noisy, turn it off
                    .with_target("goblin", LevelFilter::OFF),
            )
        }))
        .init();

    if !fb_valid {
        warn!("Unable to initialize UEFI GOP, falling back to SimpleTextProtocol");
    }
}

fn main() {
    // Setup the UEFI crate
    platform::uefi::init_uefi();
    // Set up the pre-system initialization hook
    panic::set_hook(Box::new(|panic_info| {
        runtime::panic::pre_init_panic(panic_info)
    }));

    // Initialize a Framebuffer, it *might* be empty if our GOP initialization fails
    let fb = if let Ok(gop) =
        platform::uefi::output::init_graphics(Framebuffer::MAX_WIDTH, Framebuffer::MAX_HEIGHT)
    {
        Arc::new(RwLock::new(Framebuffer::from_uefi(gop)))
    } else {
        Arc::new(RwLock::new(Framebuffer::default()))
    };

    let log_level = platform::uefi::variables::get("TAPERIPPER_LOG_LEVEL")
        .and_then(|var| tracing::Level::from_str(str::from_utf8(&var).unwrap_or("Debug")).ok())
        .or({
            platform::uefi::variables::set(
                "TAPERIPPER_LOG_LEVEL",
                DEFAULT_LOG_LEVEL.as_str().as_bytes(),
            );
            Some(DEFAULT_LOG_LEVEL)
        })
        .unwrap();

    setup_logging(&fb, log_level);

    // Now that we have logging and such, we can set the "post init" panic handler
    trace!("Setting post-init panic handler...");
    panic::set_hook(Box::new(|panic_info| {
        runtime::panic::post_init_panic(panic_info)
    }));

    #[cfg(feature = "stack-unwinding")]
    if let Err(err) = info::load_unwind_table() {
        warn!("Unable to load unwind information, stack traces on panic will not be available!");
        warn!("Error: {err:?}");
    }

    debug!("UEFI Version: {}", system::uefi_revision());
    debug!("Firmware Vendor: {}", system::firmware_vendor());
    debug!("Firmware Version: {}", system::firmware_revision());

    // Initialize ACPI and SMBIOS tables
    platform::acpi::init_tables();

    if let Some(table) = platform::uefi::tables::get_smbios() {
        debug!("SMBIOS Address: {:#018x}", table.1 as usize);
    }

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

    let mut executor = runtime::init();

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

    executor.run();

    crate::platform::uefi::system::shutdown_now();
}
