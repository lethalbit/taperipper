// SPDX-License-Identifier: MIT
// Most of this code was taken from mycelium (https://github.com/hawkw/mycelium)
// by Eliza Weisman.
//
// There has been a bit of modification, but not enough to call it substantially unique
// or novel.

use core::fmt;
use tracing_core::Metadata;

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
