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
    log::{ConsoleSubscriber, GOPConsole, QEMUDebugcon, TXTConsole},
};

fn main() {
    // Setup the UEFI crate
    crate::uefi_sys::init_uefi();
    // Hook the defaualt std panic handler
    panic::set_hook(Box::new(|pi| panic(pi)));

    // Initialize a Framebuffer, it *might* be empty if our GOP initialization fails
    let fb =
        if let Ok(gop) = uefi_sys::init_graphics(Framebuffer::MAX_WIDTH, Framebuffer::MAX_HEIGHT) {
            Arc::new(RwLock::new(Framebuffer::from_uefi(gop)))
        } else {
            Arc::new(RwLock::new(Framebuffer::default()))
        };

    // If we have a valid Framebuffer, then we can assume GOP initialized, otherwise fall back to text
    if fb.read().unwrap().is_valid() {
        fb.write().unwrap().clear_screen();

        let subscriber = ConsoleSubscriber::<GOPConsole, QEMUDebugcon>::primary_only(
            GOPConsole::from_framebuffer(fb.clone()),
        )
        .with_secondary(QEMUDebugcon {});
        if let Err(err) = tracing::subscriber::set_global_default(subscriber) {
            panic!("Unable to setup global trace handler: {:?}", err);
        }
    } else {
        let subscriber = ConsoleSubscriber::<TXTConsole, QEMUDebugcon>::new();
        if let Err(err) = tracing::subscriber::set_global_default(subscriber) {
            panic!("Unable to setup global trace handler: {:?}", err);
        }

        crate::uefi_sys::set_best_stdout_mode();

        warn!("Unable to initialize Graphics Console, falling back to Text");
    }

    debug!("UEFI Version {}", system::uefi_revision());
    debug!("Firmware Vendor  {}", system::firmware_vendor());
    debug!("Firmware Version {}", system::firmware_revision());
    debug!(
        "Image Address:  {:#018x}",
        boot::image_handle().as_ptr() as usize
    );

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
    let panic_msg = if let Some(msg) = info.payload_as_str() {
        msg
    } else {
        ""
    };

    error!("{}: {}", panic_log, panic_msg);

    loop {
        unsafe {
            asm!("hlt", options(nomem, nostack));
        }
    }
}
