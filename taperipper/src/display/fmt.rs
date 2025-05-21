// SPDX-License-Identifier: MIT
// Most of this code was taken from mycelium (https://github.com/hawkw/mycelium)
// by Eliza Weisman.
//
// There has been a bit of modification, but not enough to call it substantially unique
// or novel.
#![allow(dead_code)]

use core::fmt;
use tracing::field::DebugValue;

pub struct FormatWith<T, F = fn(&T, &mut fmt::Formatter<'_>) -> fmt::Result>
where
    F: Fn(&T, &mut fmt::Formatter<'_>) -> fmt::Result,
{
    value: T,
    fmt: F,
}

#[derive(Debug)]
pub struct WithIndent<'writer, W> {
    writer: &'writer mut W,
    indent: usize,
}

pub trait WriteExt: fmt::Write {
    /// Wraps `self` in a [`WithIndent`] writer that indents every new line
    /// that's written to it by `indent` spaces.
    #[must_use]
    #[inline]
    fn with_indent(&mut self, indent: usize) -> WithIndent<'_, Self>
    where
        Self: Sized,
    {
        WithIndent {
            writer: self,
            indent,
        }
    }
}

#[derive(Clone)]
pub struct FmtOption<'a, T> {
    opt: Option<&'a T>,
    or_else: &'a str,
}

#[inline]
#[must_use]
pub fn ptr<T: fmt::Pointer>(value: T) -> DebugValue<FormatWith<T>> {
    tracing::field::debug(FormatWith {
        value,
        fmt: fmt::Pointer::fmt,
    })
}

#[inline]
#[must_use]
pub fn hex<T: fmt::LowerHex>(value: T) -> DebugValue<FormatWith<T>> {
    tracing::field::debug(FormatWith {
        value,
        fmt: |value, f: &mut fmt::Formatter<'_>| write!(f, "{value:#x}"),
    })
}

#[must_use]
#[inline]
pub fn bin<T: fmt::Binary>(value: T) -> DebugValue<FormatWith<T>> {
    tracing::field::debug(FormatWith {
        value,
        fmt: |value, f: &mut fmt::Formatter<'_>| write!(f, "{value:#b}"),
    })
}

#[must_use]
#[inline]
pub fn alt<T: fmt::Debug>(value: T) -> DebugValue<FormatWith<T>> {
    tracing::field::debug(FormatWith {
        value,
        fmt: |value, f: &mut fmt::Formatter<'_>| write!(f, "{value:#?}"),
    })
}

#[must_use]
#[inline]
pub const fn opt<T>(value: &Option<T>) -> FmtOption<'_, T> {
    FmtOption::new(value)
}

pub fn comma_delimited<F: fmt::Display>(
    mut writer: impl fmt::Write,
    values: impl IntoIterator<Item = F>,
) -> fmt::Result {
    let mut values = values.into_iter();
    if let Some(value) = values.next() {
        write!(writer, "{value}")?;
        for value in values {
            write!(writer, ", {value}")?;
        }
    }

    Ok(())
}

impl<T, F> fmt::Debug for FormatWith<T, F>
where
    F: Fn(&T, &mut fmt::Formatter<'_>) -> fmt::Result,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.fmt)(&self.value, f)
    }
}

impl<W: fmt::Write> fmt::Write for WithIndent<'_, W> {
    fn write_str(&mut self, mut s: &str) -> fmt::Result {
        while !s.is_empty() {
            let (split, nl) = match s.find('\n') {
                Some(pos) => (pos + 1, true),
                None => (s.len(), false),
            };
            self.writer.write_str(&s[..split])?;
            if nl {
                for _ in 0..self.indent {
                    self.writer.write_char(' ')?;
                }
            }
            s = &s[split..];
        }

        Ok(())
    }
}

impl<W> WriteExt for W where W: fmt::Write {}

impl<'a, T> FmtOption<'a, T> {
    /// Returns a new `FmtOption` that formats the provided [`Option`] value.
    ///
    /// The [`fmt::opt`](opt) function can be used as shorthand for this.
    #[must_use]
    #[inline]
    pub const fn new(opt: &'a Option<T>) -> Self {
        Self {
            opt: opt.as_ref(),
            or_else: "",
        }
    }

    #[must_use]
    #[inline]
    pub fn or_else(self, or_else: &'a str) -> Self {
        Self { or_else, ..self }
    }
}

impl<T: fmt::Debug> fmt::Debug for FmtOption<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.opt {
            Some(val) => val.fmt(f),
            None => f.write_str(self.or_else),
        }
    }
}

impl<T: fmt::Display> fmt::Display for FmtOption<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.opt {
            Some(val) => val.fmt(f),
            None => f.write_str(self.or_else),
        }
    }
}

impl<T: fmt::Binary> fmt::Binary for FmtOption<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.opt {
            Some(val) => val.fmt(f),
            None => f.write_str(self.or_else),
        }
    }
}

impl<T: fmt::UpperHex> fmt::UpperHex for FmtOption<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.opt {
            Some(val) => val.fmt(f),
            None => f.write_str(self.or_else),
        }
    }
}

impl<T: fmt::LowerHex> fmt::LowerHex for FmtOption<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.opt {
            Some(val) => val.fmt(f),
            None => f.write_str(self.or_else),
        }
    }
}

impl<T: fmt::Pointer> fmt::Pointer for FmtOption<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.opt {
            Some(val) => val.fmt(f),
            None => f.write_str(self.or_else),
        }
    }
}
