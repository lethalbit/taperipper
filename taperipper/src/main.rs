// SPDX-License-Identifier: BSD-3-Clause

#![feature(uefi_std, panic_payload_as_str, panic_can_unwind)]

use core::arch::asm;
use std::{
    panic,
    sync::{Arc, RwLock},
};

use tracing::{self, debug, error, info, trace, warn};
use uefi::{
    boot::{self},
    system,
};

mod display;
mod log;
mod uefi_sys;

use crate::{
    display::framebuffer::Framebuffer,
    log::{ConsoleSubscriber, GOPConsole, QEMUDebugcon, TXTConsole, writer},
};

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

    debug!("UEFI Version {}", system::uefi_revision());
    debug!("Firmware Vendor  {}", system::firmware_vendor());
    debug!("Firmware Version {}", system::firmware_revision());
    debug!(
        "Image Address:  {:#018x}",
        boot::image_handle().as_ptr() as usize
    );
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
    crate::uefi_sys::shutdown_now();
}

pub fn panic(info: &panic::PanicHookInfo<'_>) -> ! {
    // TODO(aki): Maybe one day we'll get stack unwinding

    error!("SYSTEM PANIC");
    let panic_log = info.location().unwrap();
    let panic_msg = info.payload_as_str().unwrap_or("<No Message>");

    error!("{}: {}", panic_log, panic_msg);

    loop {
        unsafe {
            asm!("hlt", options(nomem, nostack));
        }
    }
}
