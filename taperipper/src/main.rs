// SPDX-License-Identifier: BSD-3-Clause

#![feature(uefi_std, panic_payload_as_str, panic_can_unwind)]

use core::arch::asm;
use std::panic;
mod uefi_sys;
fn main() {
    // Setup the UEFI crate
    crate::uefi_sys::init_uefi();
    // Hook the defaualt std panic handler
    panic::set_hook(Box::new(|pi| panic(pi)));
    crate::uefi_sys::shutdown_now();
}

pub fn panic(info: &panic::PanicHookInfo<'_>) -> ! {
    // TODO(aki): Maybe one day we'll get stack unwinding

    loop {
        unsafe {
            asm!("hlt", options(nomem, nostack));
        }
    }
}
