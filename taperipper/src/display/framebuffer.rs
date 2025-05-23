// SPDX-License-Identifier: BSD-3-Clause

use core::{convert, fmt, ptr, slice};

use uefi::{
    boot::ScopedProtocol,
    proto::console::gop::{self, GraphicsOutput, PixelFormat},
};

use eg_bdf::BdfTextStyle;
use embedded_graphics::{
    Drawable, Pixel,
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Point, Size},
    pixelcolor::Rgb888,
    prelude::RgbColor,
    primitives::Rectangle,
    text::Text,
};

use crate::{
    display::{font, formatting},
    uefi_sys,
};

#[derive(Clone, Copy, Debug)]
pub struct Framebuffer {
    raw_fb: *mut u8,
    x: usize,
    y: usize,
    stride: usize,
    cursor_x: usize,
    cursor_y: usize,
    pix_format: PixelFormat,
    fg_color: formatting::Color,
    bg_color: formatting::Color,
    style: formatting::Style,
}

impl formatting::SetFormatting for Framebuffer {
    fn set_fg_color(&mut self, color: formatting::Color) {
        self.fg_color = color;
    }

    fn get_fg_color(&self) -> formatting::Color {
        self.fg_color
    }

    fn set_bg_color(&mut self, color: formatting::Color) {
        self.bg_color = color;
    }

    fn get_bg_color(&self) -> formatting::Color {
        self.bg_color
    }

    fn get_style(&self) -> formatting::Style {
        self.style
    }

    fn set_style(&mut self, style: formatting::Style) {
        self.style = style
    }
}

impl Default for Framebuffer {
    fn default() -> Self {
        Self {
            raw_fb: ptr::null_mut(),
            x: 0,
            y: 0,
            stride: 0,
            cursor_x: 0,
            cursor_y: 0,
            pix_format: PixelFormat::Rgb,
            fg_color: formatting::Color::Default,
            bg_color: formatting::Color::Black,
            style: formatting::Style::None,
        }
    }
}

// SAFETY: We ain't got no threads, yolo, send it.
unsafe impl Send for Framebuffer {}
unsafe impl Sync for Framebuffer {}

impl Framebuffer {
    // WSXGA+ 1680x1050
    // NOTE(aki): For now we have a fixed max size, we should probably have some way to configure it?
    pub const MAX_WIDTH: usize = 1920;
    pub const MAX_HEIGHT: usize = 1080;

    // TODO(aki): Eventually pass this in on FB construction so we can set it via a UEFI var
    pub const FONT: &font::FramebufferFont<'static> = &font::IOSEVKAFIXED_16;

    pub fn is_valid(&self) -> bool {
        !self.raw_fb.is_null() && self.size() != 0
    }

    pub fn clear_screen(&mut self) {
        let _ = self.clear(self.bg_color.into());
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    pub fn get_raw(&mut self) -> *mut u8 {
        self.raw_fb
    }

    pub fn size(&self) -> usize {
        (self.stride * self.y) * 4
    }

    pub fn width(&self) -> usize {
        self.x
    }

    pub fn height(&self) -> usize {
        self.y
    }

    pub fn width_chars(&self) -> usize {
        self.x / Framebuffer::FONT.width()
    }

    pub fn height_chars(&self) -> usize {
        self.y / Framebuffer::FONT.height()
    }

    // TODO(aki): Maybe we should allow for the background/foreground defaults to be set?
    // Initialize a framebuffer directly from a UEFI GOP
    pub fn from_uefi(mut gfx: ScopedProtocol<GraphicsOutput>) -> Self {
        let mode = gfx.current_mode_info();
        let (width, height) = mode.resolution();

        // TODO(aki): There are some lifetime oopsies likely going on here
        Self {
            raw_fb: gfx.frame_buffer().as_mut_ptr(),
            x: width,
            y: height,
            stride: mode.stride(),
            cursor_x: 0,
            cursor_y: 0,
            pix_format: mode.pixel_format(),
            fg_color: formatting::Color::Default,
            bg_color: formatting::Color::Black,
            style: formatting::Style::None,
        }
    }

    pub fn scroll(&mut self, lines: usize) {
        let mut gop = uefi_sys::get_proto::<GraphicsOutput>().unwrap();

        // Compute the scroll region
        let src = Rectangle {
            top_left: Point {
                x: 0,
                y: (Framebuffer::FONT.height() * lines).try_into().unwrap(),
            },
            size: Size {
                width: self.x.try_into().unwrap(),
                height: (self.y - (Framebuffer::FONT.height() * lines))
                    .try_into()
                    .unwrap(),
            },
        };

        // Do a (DMA? Up to the firmware impl) GOP copy of the region up to 0
        let _ = gop.blt(gop::BltOp::VideoToVideo {
            src: (
                src.top_left.x.try_into().unwrap(),
                src.top_left.y.try_into().unwrap(),
            ),
            dest: (0, 0),
            dims: (
                src.size.width.try_into().unwrap(),
                src.size.height.try_into().unwrap(),
            ),
        });

        // fill in the now invalid video memory
        let _ = gop.blt(gop::BltOp::VideoFill {
            color: self.bg_color.into(),
            dest: (0, src.size.height.try_into().unwrap()),
            dims: (self.width(), self.height() - (src.size.height as usize)),
        });

        // make sure we adjust the cursor to the new scrolled position
        self.cursor_y -= lines;
    }
}

impl OriginDimensions for Framebuffer {
    fn size(&self) -> Size {
        Size::new(self.x.try_into().unwrap(), self.y.try_into().unwrap())
    }
}

impl DrawTarget for Framebuffer {
    type Error = convert::Infallible;
    type Color = Rgb888;

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        // We can do an accelerated fill, don't let embedded-graphics do it the slow way

        let mut gop = uefi_sys::get_proto::<GraphicsOutput>().unwrap();

        let _ = gop.blt(gop::BltOp::VideoFill {
            color: gop::BltPixel::new(color.r(), color.g(), color.b()),
            dest: (
                area.top_left.x.try_into().unwrap(),
                area.top_left.y.try_into().unwrap(),
            ),
            dims: (
                area.size.width.try_into().unwrap(),
                area.size.height.try_into().unwrap(),
            ),
        });

        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        // Same as above, but rather than a sub-region it's a full-screen fill

        let mut gop = uefi_sys::get_proto::<GraphicsOutput>().unwrap();

        let _ = gop.blt(gop::BltOp::VideoFill {
            color: gop::BltPixel::new(color.r(), color.g(), color.b()),
            dest: (0, 0),
            dims: (self.width(), self.height()),
        });

        Ok(())
    }

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        // Manually blit pixels to the GOP framebuffer the slow way
        for pixel in pixels.into_iter() {
            let pos_x = pixel.0.x as usize;
            let pos_y = pixel.0.y as usize;
            let color = pixel.1;

            if (pos_x < self.width()) && (pos_y < self.height()) {
                let offset = (pos_y * (self.stride * 4)) + (pos_x * 4);
                let raw_framebuffer =
                    unsafe { slice::from_raw_parts_mut(self.raw_fb, self.size()) };

                // Green is always in position 1
                raw_framebuffer[offset + 1] = color.g();
                // Swap B and R depending if the framebuffer is in RGB or BGR
                if self.pix_format == PixelFormat::Rgb {
                    raw_framebuffer[offset + 0] = color.r();
                    raw_framebuffer[offset + 2] = color.b();
                } else if self.pix_format == PixelFormat::Bgr {
                    raw_framebuffer[offset + 0] = color.b();
                    raw_framebuffer[offset + 2] = color.r();
                }
            }
        }
        Ok(())
    }
}

impl fmt::Write for Framebuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let text_style: BdfTextStyle<'_, Rgb888> = BdfTextStyle::new(
            Framebuffer::FONT.for_style(self.style),
            self.fg_color.into(),
        );

        // TODO(aki): Maybe we want to support more control code? (\f \v \r?)
        // TODO(aki):
        // Do we want to support some ANSI escape codes for cursor movement?
        // Supporting all ANSI codes would also mean we deal with text color
        // formatting here too.
        for line in s.split_inclusive('\n') {
            let mut line = line;

            while self.cursor_x + line.len() >= self.width_chars() {
                let end_pos = self.width_chars() - self.cursor_x;

                let text_pos = Point::new(
                    (self.cursor_x * Framebuffer::FONT.width())
                        .try_into()
                        .unwrap(),
                    ((self.cursor_y * Framebuffer::FONT.height()) + Framebuffer::FONT.height())
                        .try_into()
                        .unwrap(),
                );

                let _ = Text::new(line, text_pos, text_style)
                    .draw(self)
                    .map_err(|_| fmt::Error)?;

                self.cursor_y += 1;

                if (self.cursor_y * Framebuffer::FONT.height()) + Framebuffer::FONT.height()
                    >= self.y
                {
                    self.scroll(1);
                    self.cursor_x = 0;
                }

                line = &line[end_pos..];
            }

            // Check to see if we're writing a single newline, if so, skip it
            if line != "\n" {
                // We know the line will fit, write it
                let text_pos = Point::new(
                    (self.cursor_x * Framebuffer::FONT.width())
                        .try_into()
                        .unwrap(),
                    ((self.cursor_y * Framebuffer::FONT.height()) + Framebuffer::FONT.height())
                        .try_into()
                        .unwrap(),
                );

                let _ = Text::new(line, text_pos, text_style)
                    .draw(self)
                    .map_err(|_| fmt::Error)?;

                // Advance the column to account for the text we just wrote
                self.cursor_x += line.len();
            }

            // if we end with a newline, then advance the row cursor and reset the column cursor
            if line.ends_with('\n') {
                self.cursor_y += 1;
                self.cursor_x = 0;
            }

            // If the row cursor hits the edge of the framebuffer, force a scroll
            if (self.cursor_y * Framebuffer::FONT.height()) + Framebuffer::FONT.height() > self.y {
                self.scroll(1);
                self.cursor_x = 0;
            }
        }
        Ok(())
    }
}
