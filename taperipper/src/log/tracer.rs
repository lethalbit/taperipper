// SPDX-License-Identifier: MIT
// Most of this code was taken from mycelium (https://github.com/hawkw/mycelium)
// by Eliza Weisman.
//
// There has been a bit of modification, but not enough to call it substantially unique
// or novel.

use core::{
    fmt::{self, Write},
    sync::atomic::{AtomicU64, Ordering},
};

use tracing_core::{Event, Level, Metadata, Subscriber, field, span};
use uefi::runtime;

use crate::{
    display::{
        self,
        color::{Color, SetFormatting},
        style::{SetStyle, Style},
    },
    log::writer::{LogOutput, NoOutput},
};

#[derive(Debug)]
struct OutputConfig {
    line_length: usize,
    indent: AtomicU64,
}

#[derive(Debug)]
struct Output<W, const EN_BIT: u64> {
    writer: W,
    config: OutputConfig,
}

#[derive(Debug)]
struct Writer<'a, W: fmt::Write> {
    writer: W,
    config: &'a OutputConfig,
    current_line_len: usize,
}

#[derive(Debug)]
struct WriterPair<'a, P: fmt::Write, S: fmt::Write> {
    primary: Option<Writer<'a, P>>,
    secondary: Option<Writer<'a, S>>,
}

struct Visitor<'writer, W> {
    writer: &'writer mut W,
    seen: bool,
    newline: bool,
    comma: bool,
    altmode: bool,
}

const PRI_BIT: u64 = 1 << 0;
const SEC_BIT: u64 = 1 << 1;
const _VALID_ID_BITS: u64 = !(PRI_BIT | SEC_BIT);

pub struct ConsoleSubscriber<P, S = Option<NoOutput>> {
    pri_con: Output<P, PRI_BIT>,
    sec_con: Output<S, SEC_BIT>,
    next_id: AtomicU64,
}

impl<P> Default for ConsoleSubscriber<P>
where
    for<'a> P: LogOutput<'a> + 'static,
    for<'a> <P as LogOutput<'a>>::Writer: SetFormatting + SetStyle,
    P: Default,
{
    fn default() -> Self {
        Self::primary_only(P::default())
    }
}

impl<P, S> ConsoleSubscriber<P, S>
where
    for<'a> P: LogOutput<'a> + 'static,
    for<'a> <P as LogOutput<'a>>::Writer: SetFormatting + SetStyle,
    P: Default,
    for<'a> S: LogOutput<'a> + 'static,
    for<'a> <S as LogOutput<'a>>::Writer: SetFormatting + SetStyle,
    S: Default,
{
    pub fn new() -> Self {
        Self {
            pri_con: Output::new(P::default()),
            sec_con: Output::new(S::default()),
            next_id: AtomicU64::new(0),
        }
    }
}

impl<P, S> ConsoleSubscriber<P, S> {
    pub fn primary_only(primary: P) -> Self
    where
        for<'a> P: LogOutput<'a> + 'static,
        for<'a> S: LogOutput<'a> + 'static,
        S: Default,
    {
        Self {
            pri_con: Output::new(primary),
            sec_con: Output::new(S::default()),
            next_id: AtomicU64::new(0),
        }
    }

    pub fn with_secondary<S2>(self, output: S2) -> ConsoleSubscriber<P, S2>
    where
        for<'a> S2: LogOutput<'a> + 'static,
    {
        ConsoleSubscriber {
            pri_con: self.pri_con,
            sec_con: Output::new(output),
            next_id: self.next_id,
        }
    }
}

impl<P, S> ConsoleSubscriber<P, S> {
    fn writer<'a>(&'a self, metadata: &Metadata<'_>) -> WriterPair<'a, P::Writer, S::Writer>
    where
        P: LogOutput<'a>,
        S: LogOutput<'a>,
    {
        WriterPair {
            primary: self.pri_con.writer(metadata),
            secondary: self.sec_con.writer(metadata),
        }
    }
}

impl<P, S> Subscriber for ConsoleSubscriber<P, S>
where
    for<'a> P: LogOutput<'a> + 'static,
    for<'a> <P as LogOutput<'a>>::Writer: SetFormatting + SetStyle,
    for<'a> S: LogOutput<'a> + 'static,
    for<'a> <S as LogOutput<'a>>::Writer: SetFormatting + SetStyle,
{
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.pri_con.enabled(metadata) || self.sec_con.enabled(metadata)
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        let metadata = span.metadata();
        let id = {
            let mut id = self.next_id.fetch_add(1, Ordering::Acquire);
            if id & PRI_BIT != 0 {
                self.next_id.store(0, Ordering::Release);
            }

            if self.pri_con.enabled(metadata) {
                id |= PRI_BIT;
            }

            if self.sec_con.enabled(metadata) {
                id |= SEC_BIT;
            }

            span::Id::from_u64(id)
        };

        let mut writer = self.writer(metadata);
        let _ = write_timestamp(&mut writer);
        let _ = write_level(&mut writer, metadata.level());
        let _ = writer.indent_initial();
        let _ = writer.write_str(metadata.name());
        let _ = writer.with_fg_color(Color::BrightBlack).write_str(": ");

        self.enter(&id);
        span.record(&mut Visitor::new(&mut writer, false));
        self.exit(&id);

        id
    }

    fn record(&self, _span: &span::Id, _values: &span::Record<'_>) {}

    fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {}

    fn event(&self, event: &Event<'_>) {
        let meta = event.metadata();
        let mut writer = self.writer(meta);
        let _ = write_timestamp(&mut writer);
        let _ = write_level(&mut writer, meta.level());
        let _ = writer.indent_initial();
        let _ = write!(
            writer.with_fg_color(Color::BrightBlack),
            "{}: ",
            meta.target()
        );
        event.record(&mut Visitor::new(&mut writer, true));
    }

    fn enter(&self, span: &span::Id) {
        let bits = span.into_u64();
        self.pri_con.enter(bits);
        self.sec_con.enter(bits);
    }

    fn exit(&self, span: &span::Id) {
        let bits = span.into_u64();
        self.pri_con.exit(bits);
        self.sec_con.exit(bits);
    }
}

#[inline]
fn write_level<W>(w: &mut W, level: &Level) -> fmt::Result
where
    W: fmt::Write + SetFormatting,
{
    match *level {
        Level::TRACE => w.with_fg_color(Color::Cyan).write_str("TRACE"),
        Level::DEBUG => w.with_fg_color(Color::Magenta).write_str("DEBUG"),
        Level::INFO => w.with_fg_color(Color::Green).write_str(" INFO"),
        Level::WARN => w.with_fg_color(Color::Yellow).write_str(" WARN"),
        Level::ERROR => w.with_fg_color(Color::Red).write_str("ERROR"),
    }
}

#[inline]
fn write_timestamp<W>(w: &mut W) -> fmt::Result
where
    W: fmt::Write + SetFormatting,
{
    if let Ok(ts) = runtime::get_time() {
        write!(
            w.with_fg_color(Color::BrightBlack),
            "{:02}:{:02}:{:02} ",
            ts.hour(),
            ts.minute(),
            ts.second()
        )
    } else {
        w.with_fg_color(Color::BrightBlack).write_str("??:??:?? ")
    }
}

impl<W, const EN_BIT: u64> Output<W, EN_BIT> {
    fn new<'a>(writer: W) -> Self
    where
        W: LogOutput<'a>,
    {
        let config = OutputConfig {
            line_length: writer.line_len() - 9,
            indent: AtomicU64::new(0),
        };

        Self { writer, config }
    }

    #[inline]
    fn enabled<'a>(&'a self, metadata: &Metadata<'_>) -> bool
    where
        W: LogOutput<'a>,
    {
        self.writer.enabled(metadata)
    }

    #[inline]
    fn enter(&self, id: u64) {
        if (id & EN_BIT) != 0 {
            self.config.indent.fetch_add(1, Ordering::Release);
        }
    }

    #[inline]
    fn exit(&self, id: u64) {
        if (id & EN_BIT) != 0 {
            self.config.indent.fetch_sub(1, Ordering::Release);
        }
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

impl<P, S> SetFormatting for WriterPair<'_, P, S>
where
    P: fmt::Write + SetFormatting,
    S: fmt::Write + SetFormatting,
{
    fn set_fg_color(&mut self, color: Color) {
        if let Some(ref mut primary) = self.primary {
            primary.set_fg_color(color);
        }

        if let Some(ref mut secondary) = self.secondary {
            secondary.set_fg_color(color);
        }
    }

    fn get_fg_color(&self) -> Color {
        self.primary
            .as_ref()
            .map(SetFormatting::get_fg_color)
            .or_else(|| self.secondary.as_ref().map(SetFormatting::get_fg_color))
            .unwrap_or(Color::Default)
    }

    fn set_bg_color(&mut self, color: Color) {
        if let Some(ref mut primary) = self.primary {
            primary.set_bg_color(color);
        }

        if let Some(ref mut secondary) = self.secondary {
            secondary.set_bg_color(color);
        }
    }

    fn get_bg_color(&self) -> Color {
        self.primary
            .as_ref()
            .map(SetFormatting::get_bg_color)
            .or_else(|| self.secondary.as_ref().map(SetFormatting::get_bg_color))
            .unwrap_or(Color::Default)
    }

    fn set_colors(&mut self, fg_color: Color, bg_color: Color) {
        if let Some(ref mut primary) = self.primary {
            primary.set_colors(fg_color, bg_color);
        }

        if let Some(ref mut secondary) = self.secondary {
            secondary.set_colors(fg_color, bg_color);
        }
    }
}

impl<P, S> SetStyle for WriterPair<'_, P, S>
where
    P: fmt::Write + SetStyle,
    S: fmt::Write + SetStyle,
{
    fn set_style(&mut self, style: Style) {
        if let Some(ref mut primary) = self.primary {
            primary.set_style(style);
        }

        if let Some(ref mut secondary) = self.secondary {
            secondary.set_style(style);
        }
    }

    fn get_style(&self) -> Style {
        self.primary
            .as_ref()
            .map(SetStyle::get_style)
            .or_else(|| self.secondary.as_ref().map(SetStyle::get_style))
            .unwrap_or(Style::None)
    }
}

impl<P, S> fmt::Write for WriterPair<'_, P, S>
where
    P: fmt::Write,
    S: fmt::Write,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let err = if let Some(ref mut primary) = self.primary {
            primary.write_str(s)
        } else {
            Ok(())
        };

        if let Some(ref mut secondary) = self.secondary {
            secondary.write_str(s)?;
        }

        err
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        let err = if let Some(ref mut primary) = self.primary {
            primary.write_char(c)
        } else {
            Ok(())
        };

        if let Some(ref mut secondary) = self.secondary {
            secondary.write_char(c)?;
        }

        err
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        let err = if let Some(ref mut primary) = self.primary {
            primary.write_fmt(args)
        } else {
            Ok(())
        };

        if let Some(ref mut secondary) = self.secondary {
            secondary.write_fmt(args)?;
        }

        err
    }
}

impl<P, S> WriterPair<'_, P, S>
where
    P: fmt::Write,
    S: fmt::Write,
{
    fn indent_initial(&mut self) -> fmt::Result {
        let err = if let Some(ref mut primary) = self.primary {
            (|| {
                primary.indent()?;
                Ok(())
            })()
        } else {
            Ok(())
        };

        if let Some(ref mut secondary) = self.secondary {
            secondary.indent()?;
        }

        err
    }
}

impl<W: fmt::Write> Writer<'_, W> {
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
        // self.writer.write_str("             ")?;
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
    fn set_fg_color(&mut self, color: Color) {
        self.writer.set_fg_color(color);
    }

    fn get_fg_color(&self) -> Color {
        self.writer.get_fg_color()
    }

    fn set_bg_color(&mut self, color: Color) {
        self.writer.set_bg_color(color);
    }

    fn get_bg_color(&self) -> Color {
        self.writer.get_bg_color()
    }

    fn set_colors(&mut self, fg_color: Color, bg_color: Color) {
        self.writer.set_colors(fg_color, bg_color);
    }
}

impl<W> SetStyle for Writer<'_, W>
where
    W: fmt::Write + SetStyle,
{
    fn set_style(&mut self, style: Style) {
        self.writer.set_style(style);
    }

    fn get_style(&self) -> Style {
        self.writer.get_style()
    }
}

impl<'writer, W> Visitor<'writer, W>
where
    W: fmt::Write,
    &'writer mut W: SetFormatting + SetStyle,
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
            fn set_fg_color(&mut self, color: Color) {
                self.writer.set_fg_color(color);
            }

            #[inline]
            fn get_fg_color(&self) -> Color {
                self.writer.get_fg_color()
            }

            #[inline]
            fn set_bg_color(&mut self, color: Color) {
                self.writer.set_bg_color(color);
            }

            #[inline]
            fn get_bg_color(&self) -> Color {
                self.writer.get_bg_color()
            }

            #[inline]
            fn set_colors(&mut self, fg_color: Color, bg_color: Color) {
                self.writer.set_colors(fg_color, bg_color);
            }
        }

        impl<W: fmt::Write> SetStyle for HasWrittenNewline<'_, W>
        where
            W: SetStyle,
        {
            #[inline]
            fn set_style(&mut self, style: Style) {
                self.writer.set_style(style);
            }

            #[inline]
            fn get_style(&self) -> Style {
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
            let _ = writer.with_fg_color(Color::BrightBlack).write_char(',');
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
                let _ = writer.with_fg_color(Color::BrightBlack).write_char('.');
                let _ = writer.write_str(piece);
            }
        }

        let _ = writer.with_fg_color(Color::BrightBlack).write_char('=');
        let _ = write!(writer, "{val:?}");
        self.newline |= writer.has_written_newline;
    }
}

impl<'writer, W> field::Visit for Visitor<'writer, W>
where
    W: fmt::Write,
    &'writer mut W: SetFormatting + SetStyle,
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
