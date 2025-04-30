// SPDX-License-Identifier: BSD-3-Clause
// TODO(aki):
// We should maybe allow for setting more than one style?
// That is, if we ever support stylized output in the first place :v

use core::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Style {
    Default,
    None,
    Bold,
    Underline,
    Inverted,
}

pub trait SetStyle {
    fn set_style(&mut self, style: Style);
    fn get_style(&self) -> Style;

    fn with_bold(&mut self) -> WithBold<'_, Self>
    where
        Self: fmt::Write + Sized,
    {
        self.set_style(Style::Bold);

        WithBold { writer: self }
    }

    fn with_underline(&mut self) -> WithUnderline<'_, Self>
    where
        Self: fmt::Write + Sized,
    {
        self.set_style(Style::Underline);

        WithUnderline { writer: self }
    }

    fn with_inverted(&mut self) -> WithInverted<'_, Self>
    where
        Self: fmt::Write + Sized,
    {
        self.set_style(Style::Inverted);

        WithInverted { writer: self }
    }
}

impl<W: SetStyle> SetStyle for &'_ mut W {
    #[inline]
    fn set_style(&mut self, style: Style) {
        W::set_style(self, style);
    }

    #[inline]
    fn get_style(&self) -> Style {
        W::get_style(self)
    }
}

pub struct WithBold<'w, W>
where
    W: fmt::Write + SetStyle,
{
    writer: &'w mut W,
}

impl<W> fmt::Write for WithBold<'_, W>
where
    W: fmt::Write + SetStyle,
{
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.writer.write_str(s)
    }

    #[inline]
    fn write_char(&mut self, c: char) -> fmt::Result {
        self.writer.write_char(c)
    }

    #[inline]
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.writer.write_fmt(args)
    }
}

impl<W> Drop for WithBold<'_, W>
where
    W: fmt::Write + SetStyle,
{
    fn drop(&mut self) {
        self.writer.set_style(Style::None);
    }
}

pub struct WithUnderline<'w, W>
where
    W: fmt::Write + SetStyle,
{
    writer: &'w mut W,
}

impl<W> fmt::Write for WithUnderline<'_, W>
where
    W: fmt::Write + SetStyle,
{
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.writer.write_str(s)
    }

    #[inline]
    fn write_char(&mut self, c: char) -> fmt::Result {
        self.writer.write_char(c)
    }

    #[inline]
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.writer.write_fmt(args)
    }
}

impl<W> Drop for WithUnderline<'_, W>
where
    W: fmt::Write + SetStyle,
{
    fn drop(&mut self) {
        self.writer.set_style(Style::None);
    }
}

pub struct WithInverted<'w, W>
where
    W: fmt::Write + SetStyle,
{
    writer: &'w mut W,
}

impl<W> fmt::Write for WithInverted<'_, W>
where
    W: fmt::Write + SetStyle,
{
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.writer.write_str(s)
    }

    #[inline]
    fn write_char(&mut self, c: char) -> fmt::Result {
        self.writer.write_char(c)
    }

    #[inline]
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.writer.write_fmt(args)
    }
}

impl<W> Drop for WithInverted<'_, W>
where
    W: fmt::Write + SetStyle,
{
    fn drop(&mut self) {
        self.writer.set_style(Style::None);
    }
}
