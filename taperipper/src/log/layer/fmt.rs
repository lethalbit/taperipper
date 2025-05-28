// SPDX-License-Identifier: BSD-3-Clause
// A tracing layer much like the `tracing_subscriber` `fmt` layer for catgirl-readable
// trace logs out of things that can write strings.
//
// It's mainly templated on things like the UEFI GOP/Text console writers and in debug
// mode the QEMU debugcon IO port, but anything that implements the `LogOutput` trait
// is workable.

use core::{
    fmt::{self, Write},
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};

use tracing::{Level, Metadata, Subscriber};
use tracing_core::field;
use tracing_subscriber::{layer, registry::LookupSpan};
use uefi::runtime;

use crate::{
    display::{
        self,
        formatting::{self, SetFormatting},
    },
    log::writer::LogOutput,
};

struct OutputConfig {
    line_length: usize,
    indent: AtomicU64,
}

struct Output<W> {
    writer: W,
    config: OutputConfig,
}

struct Writer<'a, W: fmt::Write> {
    writer: W,
    config: &'a OutputConfig,
    current_line_len: usize,
}

struct Visitor<'writer, W> {
    writer: &'writer mut W,
    seen: bool,
    newline: bool,
    comma: bool,
    altmode: bool,
}

pub struct Layer<S, W> {
    writer: Output<W>,
    _inner: PhantomData<fn(S)>,
}

impl<S, W> Default for Layer<S, W>
where
    for<'a> W: LogOutput<'a> + 'static,
    for<'a> <W as LogOutput<'a>>::Writer: SetFormatting,
    W: Default,
{
    fn default() -> Self {
        Self {
            writer: Output::new(W::default()),
            _inner: PhantomData,
        }
    }
}

impl<S, W> Layer<S, W>
where
    for<'a> W: LogOutput<'a> + 'static,
    for<'a> <W as LogOutput<'a>>::Writer: SetFormatting,
    W: Default,
{
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S, W> Layer<S, W>
where
    for<'a> W: LogOutput<'a> + 'static,
    for<'a> <W as LogOutput<'a>>::Writer: SetFormatting,
    W: fmt::Write,
{
    pub fn from_writer(writer: W) -> Self {
        Self {
            writer: Output::new(writer),
            _inner: PhantomData,
        }
    }
}

impl<S, W> Layer<S, W> {
    fn writer<'a>(&'a self, metadata: &Metadata<'_>) -> Writer<'a, W::Writer>
    where
        W: LogOutput<'a>,
    {
        self.writer.writer(metadata).unwrap()
    }
}

impl<S, W> layer::Layer<S> for Layer<S, W>
where
    for<'a> W: LogOutput<'a> + 'static,
    for<'a> <W as LogOutput<'a>>::Writer: SetFormatting,
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn enabled(&self, metadata: &Metadata<'_>, _ctx: layer::Context<'_, S>) -> bool {
        self.writer.writer.enabled(metadata)
    }

    fn on_new_span(
        &self,
        attrs: &tracing_core::span::Attributes<'_>,
        _id: &tracing_core::span::Id,
        _ctx: layer::Context<'_, S>,
    ) {
        let metadata = attrs.metadata();

        let mut writer = self.writer(metadata);
        let _ = write_timestamp(&mut writer);
        let _ = write_level(&mut writer, metadata.level());
        let _ = writer.indent_initial();
        let _ = writer.write_str(metadata.name());
        let _ = writer
            .with_fg_color(formatting::Color::BrightBlack)
            .write_str(": ");
    }

    fn on_record(
        &self,
        _span: &tracing_core::span::Id,
        _values: &tracing_core::span::Record<'_>,
        _ctx: layer::Context<'_, S>,
    ) {
    }

    fn on_enter(&self, _id: &tracing_core::span::Id, _ctx: layer::Context<'_, S>) {
        self.writer.enter();
    }

    fn on_exit(&self, _id: &tracing_core::span::Id, _ctx: layer::Context<'_, S>) {
        self.writer.exit();
    }

    fn on_close(&self, _id: tracing_core::span::Id, _ctx: layer::Context<'_, S>) {}

    fn on_event(&self, event: &tracing::Event<'_>, _ctx: layer::Context<'_, S>) {
        let meta = event.metadata();
        let mut writer = self.writer(meta);
        let _ = write_timestamp(&mut writer);
        let _ = write_level(&mut writer, meta.level());
        let _ = writer.indent_initial();
        let _ = write!(
            writer.with_fg_color(formatting::Color::BrightBlack),
            "{}: ",
            meta.target()
        );

        event.record(&mut Visitor::new(&mut writer, true));
    }
}

impl<W> Output<W> {
    fn new<'a>(writer: W) -> Self
    where
        W: LogOutput<'a>,
    {
        let config = OutputConfig {
            // TODO(aki): Why do we sub (9) here?
            line_length: writer.line_len() - 9,
            indent: AtomicU64::new(0),
        };

        Self { writer, config }
    }

    #[inline]
    fn enter(&self) {
        self.config.indent.fetch_add(1, Ordering::Release);
    }

    #[inline]
    fn exit(&self) {
        self.config.indent.fetch_sub(1, Ordering::Release);
    }

    fn writer<'a>(&'a self, metadata: &Metadata<'_>) -> Option<Writer<'a, W::Writer>>
    where
        W: LogOutput<'a>,
    {
        let writer = self.writer.make_writer_for(metadata)?;
        Some(Writer {
            writer,
            config: &self.config,
            current_line_len: 0,
        })
    }
}

impl<W: fmt::Write> Writer<'_, W> {
    fn indent_initial(&mut self) -> fmt::Result {
        self.indent()
    }

    fn indent(&mut self) -> fmt::Result {
        let indent = self.config.indent.load(Ordering::Acquire);

        self.write_indent(" ")?;

        for _ in 1..=indent {
            self.write_indent(" ")?;
        }

        Ok(())
    }

    fn write_indent(&mut self, chars: &'static str) -> fmt::Result {
        self.writer.write_str(chars)?;
        self.current_line_len += chars.len();
        Ok(())
    }

    fn write_newline(&mut self) -> fmt::Result {
        self.current_line_len = 0;
        self.write_indent("              ")
    }

    fn finish(&mut self) -> fmt::Result {
        self.writer.write_char('\n')
    }
}

impl<W> fmt::Write for Writer<'_, W>
where
    W: fmt::Write,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let lines = s.split_inclusive('\n');

        for line in lines {
            let mut line = line;
            let mut loopcnt: usize = 0;

            while self.current_line_len + line.len() >= self.config.line_length {
                // If we loop more than 25 times assume we're stuck in line wrapping
                if loopcnt > 25 {
                    panic!("Line Wrapping is hard, stuck...");
                }

                let end_pos = self.config.line_length - self.current_line_len;

                // Find the right-most viable spot for doing a line break starting from
                // where we will truncate the line
                let ws_offset = line[..end_pos]
                    .chars()
                    .rev()
                    .position(|c| c.is_whitespace())
                    .unwrap_or_default();

                // If our right-most whitespace offset is `0`, then we are forced to split
                // at end_pos,
                let ws_offset = if ws_offset == 0 {
                    self.writer.write_str(&line[..end_pos])?;
                    end_pos
                } else {
                    // BUG(aki): Always force a hard-wrap, soft-wrapping is br0ken
                    // self.writer.write_str(&line[..ws_offset])?;
                    self.writer.write_str(&line[..end_pos])?;
                    end_pos
                };

                self.writer.write_char('\n')?;
                self.write_newline()?;
                self.writer.write_str(" ")?;
                self.current_line_len += 1;
                // Slice out what we might have just written
                line = &line[ws_offset..];

                loopcnt += 1;
            }

            self.writer.write_str(line)?;
            if line.ends_with('\n') {
                self.write_newline()?;
                self.writer.write_char(' ')?;
            }
            self.current_line_len += line.len();
        }

        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.writer.write_char(c)?;
        if c == '\n' {
            self.write_newline()
        } else {
            Ok(())
        }
    }
}

impl<W: fmt::Write> Drop for Writer<'_, W> {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

impl<W> SetFormatting for Writer<'_, W>
where
    W: fmt::Write + SetFormatting,
{
    fn set_fg_color(&mut self, color: formatting::Color) {
        self.writer.set_fg_color(color);
    }

    fn get_fg_color(&self) -> formatting::Color {
        self.writer.get_fg_color()
    }

    fn set_bg_color(&mut self, color: formatting::Color) {
        self.writer.set_bg_color(color);
    }

    fn get_bg_color(&self) -> formatting::Color {
        self.writer.get_bg_color()
    }

    fn set_colors(&mut self, fg_color: formatting::Color, bg_color: formatting::Color) {
        self.writer.set_colors(fg_color, bg_color);
    }

    fn set_style(&mut self, style: formatting::Style) {
        self.writer.set_style(style);
    }

    fn get_style(&self) -> formatting::Style {
        self.writer.get_style()
    }
}

impl<'writer, W> Visitor<'writer, W>
where
    W: fmt::Write,
    &'writer mut W: SetFormatting,
{
    fn new(writer: &'writer mut W, altmode: bool) -> Self {
        Self {
            writer,
            seen: false,
            newline: false,
            comma: false,
            altmode,
        }
    }

    fn record_inner(&mut self, field: &field::Field, val: &dyn fmt::Debug) {
        struct HasWrittenNewline<'a, W> {
            writer: &'a mut W,
            has_written_newline: bool,
            has_written_punct: bool,
        }

        impl<W: fmt::Write> fmt::Write for HasWrittenNewline<'_, W> {
            #[inline]
            fn write_str(&mut self, s: &str) -> fmt::Result {
                self.has_written_punct = s.ends_with(|ch: char| ch.is_ascii_punctuation());
                if s.contains('\n') {
                    self.has_written_newline = true;
                }
                self.writer.write_str(s)
            }
        }

        impl<W: fmt::Write> SetFormatting for HasWrittenNewline<'_, W>
        where
            W: SetFormatting,
        {
            #[inline]
            fn set_fg_color(&mut self, color: formatting::Color) {
                self.writer.set_fg_color(color);
            }

            #[inline]
            fn get_fg_color(&self) -> formatting::Color {
                self.writer.get_fg_color()
            }

            #[inline]
            fn set_bg_color(&mut self, color: formatting::Color) {
                self.writer.set_bg_color(color);
            }

            #[inline]
            fn get_bg_color(&self) -> formatting::Color {
                self.writer.get_bg_color()
            }

            #[inline]
            fn set_colors(&mut self, fg_color: formatting::Color, bg_color: formatting::Color) {
                self.writer.set_colors(fg_color, bg_color);
            }

            #[inline]
            fn set_style(&mut self, style: formatting::Style) {
                self.writer.set_style(style);
            }

            #[inline]
            fn get_style(&self) -> formatting::Style {
                self.writer.get_style()
            }
        }

        let mut writer = HasWrittenNewline {
            writer: &mut self.writer,
            has_written_newline: false,
            has_written_punct: false,
        };

        let nl = if self.newline { '\n' } else { ' ' };

        if field.name() == "message" {
            if self.seen {
                let _ = write!(writer, "{nl}{val:?}");
            } else {
                let _ = write!(writer, "{val:?}");
                self.comma = !writer.has_written_punct;
            }
            self.seen = true;
            return;
        }

        if self.comma {
            let _ = writer
                .with_fg_color(formatting::Color::BrightBlack)
                .write_char(',');
        }

        if self.seen {
            let _ = writer.write_char(nl);
        }

        if !self.comma {
            self.seen = true;
            self.comma = true;
        }

        // pretty-print the name with dots in the punctuation color
        let mut name_pieces = field.name().split('.');
        if let Some(piece) = name_pieces.next() {
            let _ = writer.write_str(piece);
            for piece in name_pieces {
                let _ = writer
                    .with_fg_color(formatting::Color::BrightBlack)
                    .write_char('.');
                let _ = writer.write_str(piece);
            }
        }

        let _ = writer
            .with_fg_color(formatting::Color::BrightBlack)
            .write_char('=');
        let _ = write!(writer, "{val:?}");
        self.newline |= writer.has_written_newline;
    }
}

impl<'writer, W> field::Visit for Visitor<'writer, W>
where
    W: fmt::Write,
    &'writer mut W: SetFormatting,
{
    #[inline]
    fn record_bool(&mut self, field: &field::Field, value: bool) {
        self.record_inner(field, &value);
    }

    #[inline]
    fn record_bytes(&mut self, field: &field::Field, value: &[u8]) {
        self.record_inner(field, &value);
    }

    #[inline]
    fn record_u64(&mut self, field: &field::Field, value: u64) {
        self.record_inner(field, &value)
    }

    #[inline]
    fn record_i64(&mut self, field: &field::Field, value: i64) {
        self.record_inner(field, &value)
    }

    #[inline]
    fn record_str(&mut self, field: &field::Field, value: &str) {
        if (value.len()) >= 75 {
            self.newline = true;
        }
        self.record_inner(field, &value)
    }

    fn record_debug(&mut self, field: &field::Field, value: &dyn fmt::Debug) {
        if self.altmode {
            self.record_inner(field, &display::fmt::alt(value))
        } else {
            self.record_inner(field, value)
        }
    }
}

#[inline]
fn write_level<W>(w: &mut W, level: &Level) -> fmt::Result
where
    W: fmt::Write + SetFormatting,
{
    match *level {
        Level::TRACE => w.with_fg_color(formatting::Color::Cyan).write_str("TRACE"),
        Level::DEBUG => w
            .with_fg_color(formatting::Color::Magenta)
            .write_str("DEBUG"),
        Level::INFO => w.with_fg_color(formatting::Color::Green).write_str(" INFO"),
        Level::WARN => w
            .with_fg_color(formatting::Color::Yellow)
            .write_str(" WARN"),
        Level::ERROR => w.with_fg_color(formatting::Color::Red).write_str("ERROR"),
    }
}

#[inline]
fn write_timestamp<W>(w: &mut W) -> fmt::Result
where
    W: fmt::Write + SetFormatting,
{
    if let Ok(ts) = runtime::get_time() {
        write!(
            w.with_fg_color(formatting::Color::BrightBlack),
            "{:02}:{:02}:{:02} ",
            ts.hour(),
            ts.minute(),
            ts.second()
        )
    } else {
        w.with_fg_color(formatting::Color::BrightBlack)
            .write_str("??:??:?? ")
    }
}
