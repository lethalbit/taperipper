// SPDX-License-Identifier: MIT
// Most of this code was taken from mycelium (https://github.com/hawkw/mycelium)
// by Eliza Weisman.
//
// There has been a bit of modification, but not enough to call it substantially unique
// or novel.

use core::fmt;
use tracing_core::{Level, Metadata};

use crate::display::formatting;

pub trait LogOutput<'a> {
    type Writer: fmt::Write;

    fn make_writer(&'a self) -> Self::Writer;

    #[inline]
    fn make_writer_for(&'a self, metadata: &Metadata) -> Option<Self::Writer> {
        if self.enabled(metadata) {
            return Some(self.make_writer());
        }
        None
    }

    #[inline]
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        false
    }

    #[inline]
    fn line_len(&self) -> usize {
        80
    }
}

impl<'a, F, W> LogOutput<'a> for F
where
    F: Fn() -> W,
    W: fmt::Write,
{
    type Writer = W;

    fn make_writer(&'a self) -> Self::Writer {
        (self)()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct NoOutput(());

impl fmt::Write for NoOutput {
    #[inline]
    fn write_str(&mut self, _: &str) -> fmt::Result {
        Ok(())
    }

    #[inline]
    fn write_char(&mut self, _: char) -> fmt::Result {
        Ok(())
    }

    #[inline]
    fn write_fmt(&mut self, _: fmt::Arguments<'_>) -> fmt::Result {
        Ok(())
    }
}

impl<'a> LogOutput<'a> for NoOutput {
    type Writer = Self;

    #[inline]
    fn make_writer(&'a self) -> Self::Writer {
        Self(())
    }

    #[inline]
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        false
    }

    #[inline]
    fn make_writer_for(&'a self, _: &Metadata) -> Option<Self::Writer> {
        None
    }
}

impl formatting::SetFormatting for NoOutput {
    #[inline]
    fn set_fg_color(&mut self, _: formatting::Color) {
        // NOP
    }

    #[inline]
    fn get_fg_color(&self) -> formatting::Color {
        formatting::Color::Default
    }

    #[inline]
    fn set_bg_color(&mut self, _: formatting::Color) {
        // NOP
    }

    #[inline]
    fn get_bg_color(&self) -> formatting::Color {
        formatting::Color::Default
    }

    #[inline]
    fn set_colors(&mut self, _: formatting::Color, _: formatting::Color) {
        // NOP
    }

    #[inline]
    fn set_style(&mut self, _: formatting::Style) {
        // NOP
    }

    #[inline]
    fn get_style(&self) -> formatting::Style {
        formatting::Style::None
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum EitherWriter<A, B> {
    A(A),
    B(B),
}

impl<A, B> fmt::Write for EitherWriter<A, B>
where
    A: fmt::Write,
    B: fmt::Write,
{
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self {
            EitherWriter::A(a) => a.write_str(s),
            EitherWriter::B(b) => b.write_str(s),
        }
    }

    #[inline]
    fn write_char(&mut self, c: char) -> fmt::Result {
        match self {
            EitherWriter::A(a) => a.write_char(c),
            EitherWriter::B(b) => b.write_char(c),
        }
    }

    #[inline]
    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> fmt::Result {
        match self {
            EitherWriter::A(a) => a.write_fmt(fmt),
            EitherWriter::B(b) => b.write_fmt(fmt),
        }
    }
}

impl<A, B> formatting::SetFormatting for EitherWriter<A, B>
where
    A: fmt::Write + formatting::SetFormatting,
    B: fmt::Write + formatting::SetFormatting,
{
    #[inline]
    fn set_fg_color(&mut self, color: formatting::Color) {
        match self {
            EitherWriter::A(a) => a.set_fg_color(color),
            EitherWriter::B(b) => b.set_fg_color(color),
        }
    }

    #[inline]
    fn get_fg_color(&self) -> formatting::Color {
        match self {
            EitherWriter::A(a) => a.get_fg_color(),
            EitherWriter::B(b) => b.get_fg_color(),
        }
    }

    #[inline]
    fn set_bg_color(&mut self, color: formatting::Color) {
        match self {
            EitherWriter::A(a) => a.set_bg_color(color),
            EitherWriter::B(b) => b.set_bg_color(color),
        }
    }

    #[inline]
    fn get_bg_color(&self) -> formatting::Color {
        match self {
            EitherWriter::A(a) => a.get_bg_color(),
            EitherWriter::B(b) => b.get_bg_color(),
        }
    }

    #[inline]
    fn set_colors(&mut self, fg_color: formatting::Color, bg_color: formatting::Color) {
        match self {
            EitherWriter::A(a) => a.set_colors(fg_color, bg_color),
            EitherWriter::B(b) => b.set_colors(fg_color, bg_color),
        }
    }

    #[inline]
    fn set_style(&mut self, style: formatting::Style) {
        match self {
            EitherWriter::A(a) => a.set_style(style),
            EitherWriter::B(b) => b.set_style(style),
        }
    }

    #[inline]
    fn get_style(&self) -> formatting::Style {
        match self {
            EitherWriter::A(a) => a.get_style(),
            EitherWriter::B(b) => b.get_style(),
        }
    }
}

impl<'a, W> LogOutput<'a> for Option<W>
where
    W: LogOutput<'a>,
{
    type Writer = EitherWriter<W::Writer, NoOutput>;
    #[inline]
    fn make_writer(&'a self) -> Self::Writer {
        self.as_ref()
            .map(LogOutput::make_writer)
            .map(EitherWriter::A)
            .unwrap_or(EitherWriter::B(NoOutput(())))
    }

    #[inline]
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.as_ref()
            .map(|make| make.enabled(metadata))
            .unwrap_or(false)
    }

    #[inline]
    fn make_writer_for(&'a self, metadata: &Metadata<'_>) -> Option<Self::Writer> {
        self.as_ref()
            .and_then(|make| make.make_writer_for(metadata))
            .map(EitherWriter::A)
    }

    #[inline]
    fn line_len(&self) -> usize {
        self.as_ref().map(LogOutput::line_len).unwrap_or(80)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct OrElse<P, S> {
    primary: P,
    secondary: S,
}

impl<P, S> OrElse<P, S> {
    #[allow(unused)]
    pub fn new<'a>(primary: P, secondary: S) -> Self
    where
        P: LogOutput<'a>,
        S: LogOutput<'a>,
    {
        Self { primary, secondary }
    }
}

impl<'a, P, S> LogOutput<'a> for OrElse<P, S>
where
    P: LogOutput<'a>,
    S: LogOutput<'a>,
{
    type Writer = EitherWriter<P::Writer, S::Writer>;

    #[inline]
    fn make_writer(&'a self) -> Self::Writer {
        EitherWriter::A(self.primary.make_writer())
    }

    #[inline]
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.primary.enabled(metadata) || self.secondary.enabled(metadata)
    }

    fn make_writer_for(&'a self, metadata: &Metadata) -> Option<Self::Writer> {
        self.primary
            .make_writer_for(metadata)
            .map(EitherWriter::A)
            .or_else(|| {
                self.secondary
                    .make_writer_for(metadata)
                    .map(EitherWriter::B)
            })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct WithMaxLevel<L> {
    output: L,
    level: Level,
}

impl<L> WithMaxLevel<L> {
    pub fn new(output: L, level: Level) -> Self {
        Self { output, level }
    }
}

impl<'a, L: LogOutput<'a>> LogOutput<'a> for WithMaxLevel<L> {
    type Writer = L::Writer;

    #[inline]
    fn make_writer(&'a self) -> Self::Writer {
        self.output.make_writer()
    }

    #[inline]
    fn make_writer_for(&'a self, metadata: &Metadata) -> Option<Self::Writer> {
        if self.enabled(metadata) {
            return self.output.make_writer_for(metadata);
        }

        None
    }

    #[inline]
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= &self.level && self.output.enabled(metadata)
    }

    #[inline]
    fn line_len(&self) -> usize {
        self.output.line_len()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct WithMinLevel<L> {
    output: L,
    level: Level,
}

impl<L> WithMinLevel<L> {
    #[allow(unused)]
    pub fn new(output: L, level: Level) -> Self {
        Self { output, level }
    }
}

impl<'a, L: LogOutput<'a>> LogOutput<'a> for WithMinLevel<L> {
    type Writer = L::Writer;

    #[inline]
    fn make_writer(&'a self) -> Self::Writer {
        self.output.make_writer()
    }

    #[inline]
    fn make_writer_for(&'a self, metadata: &Metadata) -> Option<Self::Writer> {
        if self.enabled(metadata) {
            return self.output.make_writer_for(metadata);
        }

        None
    }

    #[inline]
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() >= &self.level && self.output.enabled(metadata)
    }

    #[inline]
    fn line_len(&self) -> usize {
        self.output.line_len()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct WithFilter<L, F> {
    output: L,
    filter: F,
}

impl<L, F> WithFilter<L, F> {
    #[allow(unused)]
    pub fn new(output: L, filter: F) -> Self
    where
        F: Fn(&Metadata<'_>) -> bool,
    {
        Self { output, filter }
    }
}

impl<'a, L, F> LogOutput<'a> for WithFilter<L, F>
where
    L: LogOutput<'a>,
    F: Fn(&Metadata<'_>) -> bool,
{
    type Writer = L::Writer;

    #[inline]
    fn make_writer(&'a self) -> Self::Writer {
        self.output.make_writer()
    }

    #[inline]
    fn make_writer_for(&'a self, metadata: &Metadata) -> Option<Self::Writer> {
        if self.enabled(metadata) {
            return self.output.make_writer_for(metadata);
        }

        None
    }

    #[inline]
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        (self.filter)(metadata) && self.output.enabled(metadata)
    }

    #[inline]
    fn line_len(&self) -> usize {
        self.output.line_len()
    }
}
