// SPDX-License-Identifier: BSD-3-Clause

use core::fmt;

use embedded_graphics::pixelcolor::Rgb888;

use uefi::proto::console::{gop::BltPixel, text::Color as uefi_color};

// ANSI colors + RGB
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Color {
    Default,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Rgb(u8, u8, u8),
}

pub const THEME_ROSE_PINE_MOON: &[(u8, u8, u8)] = &[
    (35, 33, 54),    // #232136 | Color::Black
    (235, 111, 146), // #eb6f92 | Color::Red
    (62, 143, 176),  // #3e8fb0 | Color::Green
    (246, 193, 119), // #f6c177 | Color::Yellow
    (156, 207, 216), // #9ccfd8 | Color::Blue
    (196, 167, 231), // #c4a7e7 | Color::Magenta
    (234, 154, 151), // #ea9a97 | Color::Cyan
    (224, 222, 244), // #e0def4 | Color::White
    (110, 106, 134), // #6e6a86 | Color::BrightBlack
    (235, 111, 146), // #eb6f92 | Color::BrightRed
    (62, 143, 176),  // #3e8fb0 | Color::BrightGreen
    (246, 193, 119), // #f6c177 | Color::BrightYellow
    (156, 207, 216), // #9ccfd8 | Color::BrightBlue
    (196, 167, 231), // #c4a7e7 | Color::BrightMagenta
    (234, 154, 151), // #ea9a97 | Color::BrightCyan
    (224, 222, 244), // #e0def4 | Color::BrightWhite
];

pub trait SetFgColor {
    fn set_fg_color(&mut self, color: Color);
    fn get_fg_color(&self) -> Color;

    fn with_fg_color(&mut self, color: Color) -> WithFgColor<'_, Self>
    where
        Self: fmt::Write + Sized,
    {
        let prev_fg = self.get_fg_color();
        self.set_fg_color(color);

        WithFgColor {
            writer: self,
            old_fg_color: prev_fg,
        }
    }
}

impl<W: SetFgColor> SetFgColor for &'_ mut W {
    #[inline]
    fn set_fg_color(&mut self, color: Color) {
        W::set_fg_color(self, color);
    }

    #[inline]
    fn get_fg_color(&self) -> Color {
        W::get_fg_color(&self)
    }
}

pub trait SetBgColor {
    fn set_bg_color(&mut self, color: Color);
    fn get_bg_color(&self) -> Color;

    fn with_bg_color(&mut self, color: Color) -> WithBgColor<'_, Self>
    where
        Self: fmt::Write + Sized,
    {
        let prev_bg = self.get_bg_color();
        self.set_bg_color(color);

        WithBgColor {
            writer: self,
            old_bg_color: prev_bg,
        }
    }
}

impl<W: SetBgColor> SetBgColor for &'_ mut W {
    #[inline]
    fn set_bg_color(&mut self, color: Color) {
        W::set_bg_color(self, color);
    }

    #[inline]
    fn get_bg_color(&self) -> Color {
        W::get_bg_color(&self)
    }
}

pub trait SetColors: SetFgColor + SetBgColor {
    fn set_colors(&mut self, fg_color: Color, bg_color: Color) {
        self.set_fg_color(fg_color);
        self.set_bg_color(bg_color);
    }

    fn with_colors(&mut self, fg_color: Color, bg_color: Color) -> WithColors<'_, Self>
    where
        Self: fmt::Write + Sized,
    {
        let prev_fg = self.get_fg_color();
        let prev_bg = self.get_bg_color();
        self.set_colors(fg_color, bg_color);

        WithColors {
            writer: self,
            old_fg_color: prev_fg,
            old_bg_color: prev_bg,
        }
    }
}

impl<W: SetColors> SetColors for &'_ mut W {
    #[inline]
    fn set_colors(&mut self, fg_color: Color, bg_color: Color) {
        W::set_colors(self, fg_color, bg_color);
    }
}

pub struct WithFgColor<'w, W>
where
    W: fmt::Write + SetFgColor,
{
    writer: &'w mut W,
    old_fg_color: Color,
}

impl<W> fmt::Write for WithFgColor<'_, W>
where
    W: fmt::Write + SetFgColor,
{
    #[inline]
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.writer.write_str(s)
    }

    #[inline]
    fn write_char(&mut self, c: char) -> std::fmt::Result {
        self.writer.write_char(c)
    }
}

impl<W> Drop for WithFgColor<'_, W>
where
    W: fmt::Write + SetFgColor,
{
    fn drop(&mut self) {
        self.writer.set_fg_color(self.old_fg_color);
    }
}

pub struct WithBgColor<'w, W>
where
    W: fmt::Write + SetBgColor,
{
    writer: &'w mut W,
    old_bg_color: Color,
}

impl<W> fmt::Write for WithBgColor<'_, W>
where
    W: fmt::Write + SetBgColor,
{
    #[inline]
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.writer.write_str(s)
    }

    #[inline]
    fn write_char(&mut self, c: char) -> std::fmt::Result {
        self.writer.write_char(c)
    }
}

impl<W> Drop for WithBgColor<'_, W>
where
    W: fmt::Write + SetBgColor,
{
    fn drop(&mut self) {
        self.writer.set_bg_color(self.old_bg_color);
    }
}

pub struct WithColors<'w, W>
where
    W: fmt::Write + SetColors,
{
    writer: &'w mut W,
    old_fg_color: Color,
    old_bg_color: Color,
}

impl<W> fmt::Write for WithColors<'_, W>
where
    W: fmt::Write + SetColors,
{
    #[inline]
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.writer.write_str(s)
    }

    #[inline]
    fn write_char(&mut self, c: char) -> std::fmt::Result {
        self.writer.write_char(c)
    }
}

impl<W> Drop for WithColors<'_, W>
where
    W: fmt::Write + SetColors,
{
    fn drop(&mut self) {
        self.writer.set_colors(self.old_fg_color, self.old_bg_color);
    }
}

impl Into<uefi_color> for Color {
    fn into(self) -> uefi_color {
        match self {
            Color::Default => uefi_color::LightGray,
            Color::Black => uefi_color::Black,
            Color::Red => uefi_color::Red,
            Color::Green => uefi_color::Green,
            Color::Yellow => uefi_color::Yellow,
            Color::Blue => uefi_color::Blue,
            Color::Magenta => uefi_color::Magenta,
            Color::Cyan => uefi_color::Cyan,
            Color::White => uefi_color::White,
            Color::BrightBlack => uefi_color::LightGray,
            Color::BrightRed => uefi_color::LightRed,
            Color::BrightGreen => uefi_color::LightRed,
            Color::BrightYellow => uefi_color::Yellow,
            Color::BrightBlue => uefi_color::LightBlue,
            Color::BrightMagenta => uefi_color::LightMagenta,
            Color::BrightCyan => uefi_color::LightCyan,
            Color::BrightWhite => uefi_color::White,
            _ => uefi_color::LightGray,
        }
    }
}

#[inline]
fn _to_bltpixle(rgb: (u8, u8, u8)) -> BltPixel {
    BltPixel::new(rgb.0, rgb.1, rgb.2)
}

impl Into<BltPixel> for Color {
    fn into(self) -> BltPixel {
        match self {
            Color::Default => _to_bltpixle(THEME_ROSE_PINE_MOON[7]),
            Color::Black => _to_bltpixle(THEME_ROSE_PINE_MOON[0]),
            Color::Red => _to_bltpixle(THEME_ROSE_PINE_MOON[1]),
            Color::Green => _to_bltpixle(THEME_ROSE_PINE_MOON[2]),
            Color::Yellow => _to_bltpixle(THEME_ROSE_PINE_MOON[3]),
            Color::Blue => _to_bltpixle(THEME_ROSE_PINE_MOON[4]),
            Color::Magenta => _to_bltpixle(THEME_ROSE_PINE_MOON[5]),
            Color::Cyan => _to_bltpixle(THEME_ROSE_PINE_MOON[6]),
            Color::White => _to_bltpixle(THEME_ROSE_PINE_MOON[7]),
            Color::BrightBlack => _to_bltpixle(THEME_ROSE_PINE_MOON[8]),
            Color::BrightRed => _to_bltpixle(THEME_ROSE_PINE_MOON[9]),
            Color::BrightGreen => _to_bltpixle(THEME_ROSE_PINE_MOON[10]),
            Color::BrightYellow => _to_bltpixle(THEME_ROSE_PINE_MOON[11]),
            Color::BrightBlue => _to_bltpixle(THEME_ROSE_PINE_MOON[12]),
            Color::BrightMagenta => _to_bltpixle(THEME_ROSE_PINE_MOON[13]),
            Color::BrightCyan => _to_bltpixle(THEME_ROSE_PINE_MOON[14]),
            Color::BrightWhite => _to_bltpixle(THEME_ROSE_PINE_MOON[15]),
            Color::Rgb(r, g, b) => BltPixel::new(r, g, b),
        }
    }
}

#[inline]
fn _to_rg888(rgb: (u8, u8, u8)) -> Rgb888 {
    Rgb888::new(rgb.0, rgb.1, rgb.2)
}

impl Into<Rgb888> for Color {
    fn into(self) -> Rgb888 {
        match self {
            Color::Default => _to_rg888(THEME_ROSE_PINE_MOON[7]),
            Color::Black => _to_rg888(THEME_ROSE_PINE_MOON[0]),
            Color::Red => _to_rg888(THEME_ROSE_PINE_MOON[1]),
            Color::Green => _to_rg888(THEME_ROSE_PINE_MOON[2]),
            Color::Yellow => _to_rg888(THEME_ROSE_PINE_MOON[3]),
            Color::Blue => _to_rg888(THEME_ROSE_PINE_MOON[4]),
            Color::Magenta => _to_rg888(THEME_ROSE_PINE_MOON[5]),
            Color::Cyan => _to_rg888(THEME_ROSE_PINE_MOON[6]),
            Color::White => _to_rg888(THEME_ROSE_PINE_MOON[7]),
            Color::BrightBlack => _to_rg888(THEME_ROSE_PINE_MOON[8]),
            Color::BrightRed => _to_rg888(THEME_ROSE_PINE_MOON[9]),
            Color::BrightGreen => _to_rg888(THEME_ROSE_PINE_MOON[10]),
            Color::BrightYellow => _to_rg888(THEME_ROSE_PINE_MOON[11]),
            Color::BrightBlue => _to_rg888(THEME_ROSE_PINE_MOON[12]),
            Color::BrightMagenta => _to_rg888(THEME_ROSE_PINE_MOON[13]),
            Color::BrightCyan => _to_rg888(THEME_ROSE_PINE_MOON[14]),
            Color::BrightWhite => _to_rg888(THEME_ROSE_PINE_MOON[15]),
            Color::Rgb(r, g, b) => Rgb888::new(r, g, b),
        }
    }
}
