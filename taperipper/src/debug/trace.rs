// SPDX-License-Identifier: BSD-3-Clause
// This is a `backtrace-rs`-like module for doing rough
// stack tracing in UEFI.
// Currently it's amd64 *only* but could be expanded to
// other ISAs if needed.
#![allow(dead_code, unused_imports)]

use core::{arch::asm, ffi::c_void, fmt};

use tracing::{debug, warn};

use crate::debug::info;

#[derive(Clone)]
pub struct Frame {
    base: usize,
    ip: usize,
    sp: usize,
}

#[inline(always)]
pub fn get_ip() -> usize {
    let ip: usize;
    unsafe {
        asm!(
            "leaq (%rip), %rax",
            out("rax") ip,
            options(att_syntax, nostack)
        );
    }
    ip
}

pub struct Trace {
    start_addr: usize,
    frames: Vec<Frame>,
}

impl Trace {
    #[inline(never)]
    pub fn new() -> Trace {
        // Get the instruction pointer for this call
        // We use this to find the unwind frame then we can walk the stack
        let ip = Self::new as usize;

        // Set up frame storage
        let mut frames: Vec<Frame> = Vec::new();

        // Capture the stack pointer
        let mut sp: usize = 0;
        unsafe {
            asm!(
                "movq %rsp, %rax",
                out("rax") sp,
                options(att_syntax, nostack)
            );
        }

        let unwind_info = info::unwind_entry_for(ip);
        debug!("Unwind info for {:#018x}: {:?}", ip, unwind_info);

        warn!("Unwinding not implemented yet! Bug Aki about this!");

        // Compact the vec
        frames.shrink_to_fit();

        Self {
            start_addr: ip,
            frames,
        }
    }
}
