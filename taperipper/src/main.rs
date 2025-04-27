// SPDX-License-Identifier: BSD-3-Clause

#![feature(uefi_std, panic_payload_as_str, panic_can_unwind)]

mod uefi_sys;
fn main() {
    // Setup the UEFI crate
    crate::uefi_sys::init_uefi();
    crate::uefi_sys::shutdown_now();
}

