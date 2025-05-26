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

impl Default for Color {
    fn default() -> Self {
        Color::Default
    }
}

impl Color {
    pub fn to_ansi_fg(&self) -> &str {
        match self {
            Color::Default => "\x1b[0m",
            Color::Black => "\x1b[0;30m",
            Color::Red => "\x1b[0;31",
            Color::Green => "\x1b[0;32m",
            Color::Yellow => "\x1b[0;33m",
            Color::Blue => "\x1b[0;34m",
            Color::Magenta => "\x1b[0;35m",
            Color::Cyan => "\x1b[0;36m",
            Color::White => "\x1b[0;37m",
            Color::BrightBlack => "\x1b[0;90m",
            Color::BrightRed => "\x1b[0;91m",
            Color::BrightGreen => "\x1b[0;92m",
            Color::BrightYellow => "\x1b[0;93m",
            Color::BrightBlue => "\x1b[0;94m",
            Color::BrightMagenta => "\x1b[0;95m",
            Color::BrightCyan => "\x1b[0;96m",
            Color::BrightWhite => "\x1b[0;97m",
            _ => "\x1b[0m",
        }
    }

    pub fn to_ansi_bg(&self) -> &str {
        match self {
            Color::Default => "\x1b[0m",
            Color::Black => "\x1b[0;40m",
            Color::Red => "\x1b[0;41",
            Color::Green => "\x1b[0;42m",
            Color::Yellow => "\x1b[0;43m",
            Color::Blue => "\x1b[0;44m",
            Color::Magenta => "\x1b[0;45m",
            Color::Cyan => "\x1b[0;46m",
            Color::White => "\x1b[0;47m",
            Color::BrightBlack => "\x1b[0;100m",
            Color::BrightRed => "\x1b[0;101m",
            Color::BrightGreen => "\x1b[0;102m",
            Color::BrightYellow => "\x1b[0;103m",
            Color::BrightBlue => "\x1b[0;104m",
            Color::BrightMagenta => "\x1b[0;105m",
            Color::BrightCyan => "\x1b[0;106m",
            Color::BrightWhite => "\x1b[0;107m",
            _ => "\x1b[0m",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Style {
    Bold,
    Default,
    Inverted,
    Italic,
    None,
    Underline,
}

impl Default for Style {
    fn default() -> Self {
        Style::None
    }
}

impl Style {
    pub fn ansi_rest(&self) -> &str {
        match self {
            Style::Bold => "\x1b[22m",
            Style::Inverted => "\x1b[27m",
            Style::Italic => "\x1b[33m",
            Style::Underline => "\x1b[24m",
            Style::Default | Style::None => "\x1b[0m",
        }
    }

    pub fn to_ansi(&self) -> &str {
        match self {
            Style::Bold => "\x1b[1m",
            Style::Inverted => "\x1b[7m",
            Style::Italic => "\x1b[3m",
            Style::Underline => "\x1b[4m",
            Style::Default | Style::None => "\x1b[0m",
        }
    }
}

pub trait SetFormatting {
    fn set_fg_color(&mut self, color: Color);
    fn get_fg_color(&self) -> Color;

    fn set_bg_color(&mut self, color: Color);
    fn get_bg_color(&self) -> Color;

    fn set_colors(&mut self, fg_color: Color, bg_color: Color) {
        self.set_fg_color(fg_color);
        self.set_bg_color(bg_color);
    }

    fn set_style(&mut self, style: Style);
    fn get_style(&self) -> Style;

    #[allow(unused)]
    fn with_fg_color(&mut self, color: Color) -> WithFormatting<'_, Self>
    where
        Self: fmt::Write + Sized,
    {
        let prev_fg = self.get_fg_color();
        self.set_fg_color(color);

        WithFormatting {
            writer: self,
            prev_fg_color: Some(prev_fg),
            prev_bg_color: None,
            prev_style: None,
        }
    }

    #[allow(unused)]
    fn with_bg_color(&mut self, color: Color) -> WithFormatting<'_, Self>
    where
        Self: fmt::Write + Sized,
    {
        let prev_bg = self.get_bg_color();
        self.set_bg_color(color);

        WithFormatting {
            writer: self,
            prev_fg_color: None,
            prev_bg_color: Some(prev_bg),
            prev_style: None,
        }
    }

    #[allow(unused)]
    fn with_colors(&mut self, fg_color: Color, bg_color: Color) -> WithFormatting<'_, Self>
    where
        Self: fmt::Write + Sized,
    {
        let prev_fg = self.get_fg_color();
        let prev_bg = self.get_bg_color();
        self.set_colors(fg_color, bg_color);

        WithFormatting {
            writer: self,
            prev_fg_color: Some(prev_fg),
            prev_bg_color: Some(prev_bg),
            prev_style: None,
        }
    }

    #[allow(unused)]
    fn with_bold(&mut self) -> WithFormatting<'_, Self>
    where
        Self: fmt::Write + Sized,
    {
        let prev = self.get_style();
        self.set_style(Style::Bold);

        WithFormatting {
            writer: self,
            prev_fg_color: None,
            prev_bg_color: None,
            prev_style: Some(prev),
        }
    }

    #[allow(unused)]
    fn with_underline(&mut self) -> WithFormatting<'_, Self>
    where
        Self: fmt::Write + Sized,
    {
        let prev = self.get_style();
        self.set_style(Style::Underline);

        WithFormatting {
            writer: self,
            prev_fg_color: None,
            prev_bg_color: None,
            prev_style: Some(prev),
        }
    }

    #[allow(unused)]
    fn with_inverted(&mut self) -> WithFormatting<'_, Self>
    where
        Self: fmt::Write + Sized,
    {
        let prev = self.get_style();
        self.set_style(Style::Inverted);

        WithFormatting {
            writer: self,
            prev_fg_color: None,
            prev_bg_color: None,
            prev_style: Some(prev),
        }
    }

    #[allow(unused)]
    fn with_italic(&mut self) -> WithFormatting<'_, Self>
    where
        Self: fmt::Write + Sized,
    {
        let prev = self.get_style();
        self.set_style(Style::Italic);

        WithFormatting {
            writer: self,
            prev_fg_color: None,
            prev_bg_color: None,
            prev_style: Some(prev),
        }
    }
}

impl<W: SetFormatting> SetFormatting for &'_ mut W {
    #[inline]
    fn set_fg_color(&mut self, color: Color) {
        W::set_fg_color(self, color);
    }

    #[inline]
    fn get_fg_color(&self) -> Color {
        W::get_fg_color(self)
    }

    #[inline]
    fn set_bg_color(&mut self, color: Color) {
        W::set_bg_color(self, color);
    }

    #[inline]
    fn get_bg_color(&self) -> Color {
        W::get_bg_color(self)
    }

    #[inline]
    fn set_colors(&mut self, fg_color: Color, bg_color: Color) {
        W::set_colors(self, fg_color, bg_color);
    }

    #[inline]
    fn set_style(&mut self, style: Style) {
        W::set_style(self, style);
    }

    #[inline]
    fn get_style(&self) -> Style {
        W::get_style(self)
    }
}

pub struct WithFormatting<'w, W>
where
    W: fmt::Write + SetFormatting,
{
    writer: &'w mut W,
    prev_fg_color: Option<Color>,
    prev_bg_color: Option<Color>,
    prev_style: Option<Style>,
}

impl<W> fmt::Write for WithFormatting<'_, W>
where
    W: fmt::Write + SetFormatting,
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

impl<W> Drop for WithFormatting<'_, W>
where
    W: fmt::Write + SetFormatting,
{
    fn drop(&mut self) {
        if let Some(fg_color) = self.prev_fg_color {
            self.writer.set_fg_color(fg_color);
        }

        if let Some(bg_color) = self.prev_bg_color {
            self.writer.set_bg_color(bg_color);
        }

        if let Some(style) = self.prev_style {
            self.writer.set_style(style);
        }
    }
}

impl From<Color> for uefi_color {
    fn from(color: Color) -> Self {
        match color {
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

impl From<Color> for BltPixel {
    fn from(color: Color) -> Self {
        match color {
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

impl From<Color> for Rgb888 {
    fn from(color: Color) -> Self {
        match color {
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
