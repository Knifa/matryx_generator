
use log2::*;
use palette::{rgb::Rgb, FromColor, Hsl, IntoColor, Lch, ShiftHue, Srgb};
use std::{
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
    thread::{self, sleep},
    time::{self, Duration, Instant},
};

use v4l::buffer::Type;
use v4l::io::traits::CaptureStream;
use v4l::prelude::*;
use v4l::video::Capture;
use v4l::Device;
use v4l::FourCC;

use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{Dimensions, Point, Size},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::Rectangle,
    Pixel,
};

#[derive(Clone)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        Canvas {
            width,
            height,
            pixels: vec![0; (width * height * 3) as usize],
        }
    }

    pub fn clear(&mut self) {
        for pixel in self.pixels.iter_mut() {
            *pixel = 0;
        }
    }

    pub fn clear_with_color(&mut self, r: f32, g: f32, b: f32) {
        for y in 0..self.height {
            for x in 0..self.width {
                let index = ((y * self.width + x) * 3) as usize;
                self.pixels[index] = (r * 255.0) as u8;
                self.pixels[index + 1] = (g * 255.0) as u8;
                self.pixels[index + 2] = (b * 255.0) as u8;
            }
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, r: f32, g: f32, b: f32) {
        let index = ((y * self.width + x) * 3) as usize;
        self.pixels[index] = (r * 255.0) as u8;
        self.pixels[index + 1] = (g * 255.0) as u8;
        self.pixels[index + 2] = (b * 255.0) as u8;
    }

    pub fn get_pixel(&mut self, x: u32, y: u32) -> [f32; 3] {
        let index = ((y * self.width + x) * 3) as usize;
        let r = self.pixels[index] as f32 / 255.0;
        let g = self.pixels[index + 1] as f32 / 255.0;
        let b = self.pixels[index + 2] as f32 / 255.0;
        return [r as f32, g as f32, b as f32];
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

impl Dimensions for Canvas {
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(
            Point::new(0, 0),
            Size::new(self.width as u32, self.height as u32),
        )
    }
}

impl DrawTarget for Canvas {
    type Color = Rgb888;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for px in pixels {
            self.set_pixel(
                px.0.x as u32,
                px.0.y as u32,
                px.1.r() as f32,
                px.1.g() as f32,
                px.1.b() as f32,
            );
            // self.set(px.0.x, px.0.y, &px.1.into());
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.clear_with_color(color.r() as f32, color.g() as f32, color.b() as f32);
        Ok(())
    }
}

pub fn filter_background(canvas: &mut Canvas, canvas2: &mut Canvas) {
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let curr_pixel = canvas2.get_pixel(x, y);
            if curr_pixel[0] != 0.0 && curr_pixel[1] != 0.0 && curr_pixel[2] != 0.0 {
                let curr_pixel2 = canvas.get_pixel(x, y);
                canvas.set_pixel(x, y, curr_pixel2[0], curr_pixel2[1], curr_pixel2[2]);
            } else {
                canvas.set_pixel(x, y, curr_pixel[0], curr_pixel[1], curr_pixel[2]);
            }
        }
    }
}

pub fn filter_foreground(canvas: &mut Canvas, canvas2: &mut Canvas) {
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let curr_pixel2 = canvas.get_pixel(x, y);
            if curr_pixel2[0] != 0.0 && curr_pixel2[1] != 0.0 && curr_pixel2[2] != 0.0 {
                canvas.set_pixel(x, y, curr_pixel2[0], curr_pixel2[1], curr_pixel2[2]);
            } else {
                let curr_pixel = canvas2.get_pixel(x, y);
                canvas.set_pixel(x, y, curr_pixel[0], curr_pixel[1], curr_pixel[2]);
            }
        }
    }
}

pub fn color_lightness(curr_pixel: [f32; 3], lightness: f32) -> Rgb {
    let my_rgb = Srgb::new(curr_pixel[0], curr_pixel[1], curr_pixel[2]);
    let my_lch = Lch::from_color(my_rgb);
    let mut my_hsl: Hsl = my_lch.into_color();
    my_hsl.lightness *= lightness;
    return Srgb::from_color(my_hsl);
}

pub fn filter_bright_foreground(canvas: &mut Canvas, canvas2: &mut Canvas, lightness: f32) {
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let curr_pixel2 = canvas.get_pixel(x, y);
            let curr_pixel = canvas2.get_pixel(x, y);
            if curr_pixel2[0] != 0.0 && curr_pixel2[1] != 0.0 && curr_pixel2[2] != 0.0 {
                canvas.set_pixel(x, y, curr_pixel[0], curr_pixel[1], curr_pixel[2]);
                // lighten?
                // let my_rgb = color_lightness(curr_pixel, 1.0);
                // canvas.set_pixel(x, y, my_rgb.red, /my_rgb.green, my_rgb.blue);
            } else {
                // darken
                let my_rgb = color_lightness(curr_pixel, lightness);
                canvas.set_pixel(x, y, my_rgb.red, my_rgb.green, my_rgb.blue);
            }
        }
    }
}

pub fn filter_bright_background(canvas: &mut Canvas, canvas2: &mut Canvas, lightness: f32) {
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let curr_pixel2 = canvas2.get_pixel(x, y);
            let curr_pixel = canvas.get_pixel(x, y);
            if curr_pixel2[0] == 0.0 && curr_pixel2[1] == 0.0 && curr_pixel2[2] == 0.0 {
                // canvas.set_pixel(x, y, curr_pixel[0], curr_pixel[1], curr_pixel[2]);
                // lighten?
                // let my_rgb = color_lightness(curr_pixel, 1.0);
                // canvas.set_pixel(x, y, my_rgb.red, /my_rgb.green, my_rgb.blue);
            } else {
                // darken
                let my_rgb = color_lightness(curr_pixel, lightness);
                canvas.set_pixel(x, y, my_rgb.red, my_rgb.green, my_rgb.blue);
            }
        }
    }
}

pub fn filter_darken(canvas: &mut Canvas, lightness: f32) {
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let curr_pixel = canvas.get_pixel(x, y);
            let my_rgb = color_lightness(curr_pixel, lightness);
            canvas.set_pixel(x, y, my_rgb.red, 0.0, 0.0);
        }
    }
}

pub fn filter_red(canvas: &mut Canvas) {
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let curr_pixel = canvas.get_pixel(x, y);
            canvas.set_pixel(x, y, curr_pixel[0], 0.0, 0.0);
        }
    }
}

pub fn filter_hue_shift(canvas: &mut Canvas, shift: f32) {
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let curr_pixel = canvas.get_pixel(x, y);

            let my_rgb = Srgb::new(curr_pixel[0], curr_pixel[1], curr_pixel[2]);
            let hue_shifted = Lch::from_color(my_rgb).shift_hue(shift);
            let new_pixel = Srgb::from_color(hue_shifted);
            canvas.set_pixel(x, y, new_pixel.red, new_pixel.green, new_pixel.blue);
        }
    }
}
