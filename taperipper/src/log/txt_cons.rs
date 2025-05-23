// SPDX-License-Identifier: BSD-3-Clause
// This is a logging interface for the tracing subscriber that emits
// the log messages via the UEFI `SimpleTextProtocol`'s stdout.
//
// The UEFI text protocol is slow, and clunky, and all around painful
// but it works as a stop-gap until GOP-based consols can be set up.
//
// Unlike the QEMU Debugcon interface, this one actually supports colors!
// but that's about it, you get some fixed colors and no extra formatting.

use core::{
    fmt,
    sync::atomic::{AtomicPtr, Ordering},
};
use tracing::Metadata;
use uefi::{proto::console::text::Output, table};

use crate::{
    display::{formatting, style},
    log::writer,
    uefi_sys,
};

// TODO(aki): Should probably replace AtomicPtr<> with an Arc<Mutex<>>...
pub struct TXTConsole {
    writer: AtomicPtr<Output>,
    _style: style::Style,
    _fg_color: formatting::Color,
    _bg_color: formatting::Color,
}

impl Clone for TXTConsole {
    fn clone(&self) -> Self {
        Self {
            writer: AtomicPtr::new(self.writer.load(Ordering::Acquire)),
            _style: style::Style::None,
            _fg_color: formatting::Color::Default,
            _bg_color: formatting::Color::Black,
        }
    }
}

impl Default for TXTConsole {
    fn default() -> Self {
        let system_table = table::system_table_raw().unwrap();
        let system_table = unsafe { system_table.as_ref() };

        let stdout: *mut Output = system_table.stdout.cast();

        Self {
            writer: AtomicPtr::new(stdout),
            _style: style::Style::None,
            _fg_color: formatting::Color::Default,
            _bg_color: formatting::Color::Black,
        }
    }
}

impl TXTConsole {
    pub fn new() -> Self {
        TXTConsole::default()
    }

    #[must_use]
    fn output(&self) -> *mut Output {
        self.writer.load(Ordering::Acquire)
    }
}

impl<'a> writer::LogOutput<'a> for TXTConsole {
    type Writer = Self;

    #[inline]
    fn make_writer(&'a self) -> Self::Writer {
        TXTConsole::new()
    }

    #[inline]
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        uefi_sys::has_boot_services() && !self.output().is_null()
    }

    #[inline]
    fn line_len(&self) -> usize {
        let output = unsafe { self.output().as_ref().unwrap() };

        if let Some(mode) = output.current_mode().unwrap() {
            mode.columns()
        } else {
            80
        }
    }
}

impl fmt::Write for TXTConsole {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let output = unsafe { self.output().as_mut().unwrap() };

        output.write_str(s)
    }

    #[inline]
    fn write_char(&mut self, c: char) -> fmt::Result {
        let output = unsafe { self.output().as_mut().unwrap() };
        output.write_char(c)
    }

    #[inline]
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        let output = unsafe { self.output().as_mut().unwrap() };
        output.write_fmt(args)
    }
}

impl formatting::SetFormatting for TXTConsole {
    #[inline]
    fn set_fg_color(&mut self, color: formatting::Color) {
        self._fg_color = color;

        unsafe {
            let _ = self
                .output()
                .as_mut()
                .unwrap()
                .set_color(color.into(), self._bg_color.into());
        }
    }

    #[inline]
    fn get_fg_color(&self) -> formatting::Color {
        self._fg_color
    }

    #[inline]
    fn set_bg_color(&mut self, color: formatting::Color) {
        self._bg_color = color;

        unsafe {
            let _ = self
                .output()
                .as_mut()
                .unwrap()
                .set_color(self._fg_color.into(), color.into());
        }
    }

    #[inline]
    fn get_bg_color(&self) -> formatting::Color {
        self._bg_color
    }
}

impl style::SetStyle for TXTConsole {
    #[inline]
    fn set_style(&mut self, _style: style::Style) {
        // NOP
    }

    #[inline]
    fn get_style(&self) -> style::Style {
        style::Style::None
    }
}
