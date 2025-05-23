// SPDX-License-Identifier: BSD-3-Clause

use eg_bdf::BdfFont;

use iosevka_embedded::{
    IOSEVKAFIXED_EXTENDEDBOLD_8, IOSEVKAFIXED_EXTENDEDBOLD_16, IOSEVKAFIXED_EXTENDEDBOLD_24,
    IOSEVKAFIXED_EXTENDEDBOLD_32, IOSEVKAFIXED_EXTENDEDTHIN_8, IOSEVKAFIXED_EXTENDEDTHIN_16,
    IOSEVKAFIXED_EXTENDEDTHIN_24, IOSEVKAFIXED_EXTENDEDTHIN_32,
};

use crate::display::formatting;

pub struct FramebufferFont<'a> {
    normal: BdfFont<'a>,
    bold: BdfFont<'a>,
    height: usize,
    width: usize,
}

impl<'a> FramebufferFont<'a> {
    pub const fn new(normal: BdfFont<'a>, bold: BdfFont<'a>) -> Self {
        Self {
            bold: bold,
            height: (normal.ascent + normal.descent) as usize,
            // XXX(aki): This is kinda janky but it's monospace so should be the same for all
            width: normal.glyphs[0].device_width as usize,
            normal: normal,
        }
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn for_style(&self, style: formatting::Style) -> &BdfFont<'a> {
        match style {
            formatting::Style::Bold => &self.bold,
            _ => &self.normal,
        }
    }

    pub fn normal(&self) -> &BdfFont<'a> {
        &self.normal
    }

    pub fn bold(&self) -> &BdfFont<'a> {
        &self.normal
    }
}

pub const IOSEVKAFIXED_8: FramebufferFont<'static> =
    FramebufferFont::new(IOSEVKAFIXED_EXTENDEDTHIN_8, IOSEVKAFIXED_EXTENDEDBOLD_8);
pub const IOSEVKAFIXED_16: FramebufferFont<'static> =
    FramebufferFont::new(IOSEVKAFIXED_EXTENDEDTHIN_16, IOSEVKAFIXED_EXTENDEDBOLD_16);
pub const IOSEVKAFIXED_24: FramebufferFont<'static> =
    FramebufferFont::new(IOSEVKAFIXED_EXTENDEDTHIN_24, IOSEVKAFIXED_EXTENDEDBOLD_24);
pub const IOSEVKAFIXED_32: FramebufferFont<'static> =
    FramebufferFont::new(IOSEVKAFIXED_EXTENDEDTHIN_32, IOSEVKAFIXED_EXTENDEDBOLD_32);
