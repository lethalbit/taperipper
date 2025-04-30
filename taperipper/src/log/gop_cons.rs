// SPDX-License-Identifier: BSD-3-Clause

use core::fmt;

use std::sync::{Arc, RwLock};
use tracing::Metadata;
use uefi::{boot::ScopedProtocol, proto::console::gop::GraphicsOutput};

use crate::{
    display::{color, framebuffer::Framebuffer, style},
    log::writer,
};

pub struct GOPConsole {
    framebuffer: Arc<RwLock<Framebuffer>>,
}

impl GOPConsole {
    pub fn new() -> Self {
        Self {
            framebuffer: Arc::new(RwLock::new(Framebuffer::default())),
        }
    }

    pub fn from_framebuffer(framebuffer: Arc<RwLock<Framebuffer>>) -> Self {
        Self { framebuffer }
    }

    pub fn from_uefi(gfx: ScopedProtocol<GraphicsOutput>) -> Self {
        Self {
            framebuffer: Arc::new(RwLock::new(Framebuffer::from_uefi(gfx))),
        }
    }
}

impl Clone for GOPConsole {
    fn clone(&self) -> Self {
        Self {
            framebuffer: self.framebuffer.clone(),
        }
    }
}

impl<'a> writer::LogOutput<'a> for GOPConsole {
    type Writer = Self;

    #[inline]
    fn make_writer(&'a self) -> Self::Writer {
        Self {
            framebuffer: self.framebuffer.clone(),
        }
    }

    #[inline]
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        !self.framebuffer.write().unwrap().get_raw().is_null()
    }

    #[inline]
    fn line_len(&self) -> usize {
        self.framebuffer.read().unwrap().width_chars()
    }
}

impl fmt::Write for GOPConsole {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.framebuffer.write().unwrap().write_str(s)
    }
}

impl color::SetFgColor for GOPConsole {
    #[inline]
    fn set_fg_color(&mut self, color: color::Color) {
        self.framebuffer.write().unwrap().set_fg_color(color);
    }

    #[inline]
    fn get_fg_color(&self) -> color::Color {
        self.framebuffer.read().unwrap().get_fg_color()
    }
}

impl color::SetBgColor for GOPConsole {
    #[inline]
    fn set_bg_color(&mut self, color: color::Color) {
        self.framebuffer.write().unwrap().set_bg_color(color);
    }

    #[inline]
    fn get_bg_color(&self) -> color::Color {
        self.framebuffer.read().unwrap().get_bg_color()
    }
}

impl color::SetColors for GOPConsole {}

impl style::SetStyle for GOPConsole {
    #[inline]
    fn set_style(&mut self, _style: style::Style) {
        // NOP
    }

    #[inline]
    fn get_style(&self) -> style::Style {
        style::Style::None
    }
}
