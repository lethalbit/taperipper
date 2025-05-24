// SPDX-License-Identifier: BSD-3-Clause

use core::{arch::asm, fmt, marker::PhantomData};
use std::cmp::Ordering;

#[derive(Clone, Copy)]
pub struct Msr {
    pub reg: u32,
    name: Option<&'static str>,
    _t: PhantomData<()>,
}

impl Msr {
    pub const fn new(reg: u32) -> Self {
        Self {
            reg: reg,
            name: None,
            _t: PhantomData,
        }
    }

    pub const fn with_name(reg: u32, name: &'static str) -> Self {
        Self {
            reg: reg,
            name: Some(name),
            _t: PhantomData,
        }
    }

    #[must_use]
    pub fn read(&self) -> u64 {
        let (high, low): (u32, u32);
        unsafe {
            asm!(
                "rdmsr",
                in("ecx") self.reg,
                out("eax") low,
                out("edx") high,
                options(att_syntax, nomem, nostack, preserves_flags)
            )
        }
        ((high as u64) << 32) | (low as u64)
    }

    pub fn write(&self, value: u64) {
        let low = value as u32;
        let high = (value >> 32) as u32;
        unsafe {
            asm!(
                "wrmsr",
                in("ecx") self.reg,
                in("eax") low,
                in("edx") high,
                options(att_syntax, nomem, nostack)
            )
        }
    }
}

impl fmt::Debug for Msr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { reg, name, .. } = self;
        if let Some(name) = name {
            write!(f, "Msr({reg:#09x}, {name})")
        } else {
            write!(f, "Msr({reg:#09x})")
        }
    }
}

impl fmt::Display for Msr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { reg, name, .. } = self;
        if let Some(name) = name {
            write!(f, "MSR: {reg:#09x} ({name})")
        } else {
            write!(f, "MSR: {reg:#09x}")
        }
    }
}

impl PartialEq for Msr {
    fn eq(&self, other: &Self) -> bool {
        self.reg == other.reg
    }
}

impl Eq for Msr {}

impl PartialOrd for Msr {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.reg > other.reg {
            Some(Ordering::Greater)
        } else if self.reg < other.reg {
            Some(Ordering::Less)
        } else if self == other {
            Some(Ordering::Equal)
        } else {
            None
        }
    }
}

// NOTE(aki): The only MSR we use at the moment, might need more eventually:tm:
pub const GS_BASE: Msr = Msr::with_name(0xC0000101, "GS Base");
