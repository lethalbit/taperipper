// SPDX-License-Identifier: BSD-3-Clause
// This is a logging interface for the tracing subscriber that emits
// the log messages into the QEMU Debugcon device.
//
// Using this as the secondary fallback logger lets us see log messages if somehow
// the primary logging output fails.
//
// The default IO port is `0xE9` as noted in the qemu documentation, but it can be
// remapped.

use core::{arch::asm, fmt};
use std::fmt::Write;
use tracing::Metadata;

use crate::{
    display::formatting,
    log::{layer, writer},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct QEMUDebugcon {
    fg: formatting::Color,
    bg: formatting::Color,
    style: formatting::Style,
}

impl QEMUDebugcon {
    const PORT: u16 = 0xE9;
}

impl<'a> writer::LogOutput<'a> for QEMUDebugcon {
    type Writer = Self;

    #[inline]
    fn make_writer(&'a self) -> Self::Writer {
        QEMUDebugcon {
            fg: formatting::Color::Default,
            bg: formatting::Color::Default,
            style: formatting::Style::None,
        }
    }

    #[cfg(debug_assertions)]
    #[inline]
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    #[cfg(not(debug_assertions))]
    #[inline]
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        false
    }

    #[inline]
    fn line_len(&self) -> usize {
        130
    }
}

#[cfg(debug_assertions)]
impl fmt::Write for QEMUDebugcon {
    #[inline]
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        for &byte in s.as_bytes() {
            unsafe {
                asm!("outb %al, %dx", in("al") byte, in("dx") Self::PORT, options(att_syntax));
            }
        }
        Ok(())
    }

    #[inline]
    fn write_char(&mut self, c: char) -> std::fmt::Result {
        let mut bytes = [0; 4];
        c.encode_utf8(&mut bytes);

        for &byte in bytes[0..c.len_utf8()].iter() {
            unsafe {
                asm!("outb %al, %dx", in("al") byte, in("dx") Self::PORT, options(att_syntax));
            }
        }

        Ok(())
    }
}

#[cfg(not(debug_assertions))]
impl fmt::Write for QEMUDebugcon {
    #[inline]
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        // NOP
        Ok(())
    }
}

impl formatting::SetFormatting for QEMUDebugcon {
    #[inline]
    fn set_fg_color(&mut self, color: formatting::Color) {
        self.fg = color;
        let _ = self.write_str(color.as_ansi_fg());
    }

    #[inline]
    fn get_fg_color(&self) -> formatting::Color {
        self.fg
    }

    #[inline]
    fn set_bg_color(&mut self, color: formatting::Color) {
        self.bg = color;
        let _ = self.write_str(color.as_ansi_bg());
    }

    #[inline]
    fn get_bg_color(&self) -> formatting::Color {
        self.bg
    }

    #[inline]
    fn set_style(&mut self, style: formatting::Style) {
        let old = self.style;
        self.style = style;
        let _ = match old {
            formatting::Style::Default => self.write_str(old.ansi_rest()),
            _ => self.write_str(style.as_ansi()),
        };
    }

    #[inline]
    fn get_style(&self) -> formatting::Style {
        formatting::Style::None
    }
}

pub fn layer<S>() -> layer::fmt::Layer<S, QEMUDebugcon> {
    layer::fmt::Layer::<S, QEMUDebugcon>::default()
}
