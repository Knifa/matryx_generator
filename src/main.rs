mod scenes;

use led_matrix_zmq::client::{MatrixClient, MatrixClientSettings};
use palette::{rgb::Rgb, FromColor, Hsl, IntoColor, Lch, Srgb};
use std::time;

use scenes::{ClockScene, PlasmaScene, WaveScene};

use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{Dimensions, Point, Size},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::Rectangle,
    Pixel,
};

const FRAME_TIME: time::Duration = time::Duration::from_millis((1000 / 30) as u64);

struct FrameTimer {
    prev_tick: Option<FrameTick>,
}

#[derive(Copy, Clone, Debug)]
struct FrameTick {
    start: time::Instant,
    instant: time::Instant,
    delta: time::Duration,

    t: f32,
    dt: f32,
}

impl FrameTick {
    fn from_start() -> FrameTick {
        let now = time::Instant::now();

        FrameTick {
            start: now,
            instant: now,
            delta: time::Duration::from_millis(0),
            t: 0.0,
            dt: 0.0,
        }
    }

    fn from_prev(last_tick: &FrameTick) -> FrameTick {
        let start = last_tick.start;
        let instant = time::Instant::now();
        let delta = last_tick.instant.elapsed();
        let t = start.elapsed().as_secs_f32();
        let dt = delta.as_secs_f32();

        FrameTick {
            start,
            instant,
            delta,
            t,
            dt,
        }
    }
}

impl FrameTimer {
    fn new() -> Self {
        FrameTimer { prev_tick: None }
    }

    fn tick(&mut self) -> FrameTick {
        if self.prev_tick.is_none() {
            self.prev_tick = Some(FrameTick::from_start());
            return self.prev_tick.unwrap();
        } else {
            self.prev_tick = Some(FrameTick::from_prev(self.prev_tick.as_ref().unwrap()))
        }

        self.prev_tick.unwrap()
    }

    fn wait_for_next_frame(&self) {
        if self.prev_tick.is_none() {
            return;
        }

        let delta = self.prev_tick.unwrap().instant.elapsed();
        if delta < FRAME_TIME {
            std::thread::sleep(FRAME_TIME - delta);
        }
    }
}

trait Scene {
    fn tick(&mut self, _canvas: &mut Canvas, _tick: &FrameTick) {}
}

#[derive(Clone)]
pub struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Canvas {
    fn new(width: u32, height: u32) -> Self {
        Canvas {
            width,
            height,
            pixels: vec![0; (width * height * 3) as usize],
        }
    }

    fn clear(&mut self) {
        for pixel in self.pixels.iter_mut() {
            *pixel = 0;
        }
    }

    fn clear_with_color(&mut self, r: f32, g: f32, b: f32) {
        for y in 0..self.height {
            for x in 0..self.width {
                let index = (y * self.width + x) * 3;
                self.pixels[index as usize] = (r * 255.0) as u8;
                self.pixels[index as usize + 1] = (g * 255.0) as u8;
                self.pixels[index as usize + 2] = (b * 255.0) as u8;
            }
        }
    }

    fn set_pixel(&mut self, x: u32, y: u32, r: f32, g: f32, b: f32) {
        let index = ((y * self.width + x) * 3) as usize;
        self.pixels[index] = (r * 255.0) as u8;
        self.pixels[index + 1] = (g * 255.0) as u8;
        self.pixels[index + 2] = (b * 255.0) as u8;
    }

    fn get_pixel(&mut self, x: u32, y: u32) -> [f32; 3] {
        let index = ((y * self.width + x) * 3) as usize;
        let r = self.pixels[index] as f32 / 255.0;
        let g = self.pixels[index + 1] as f32 / 255.0;
        let b = self.pixels[index + 2] as f32 / 255.0;
        return [r as f32, g as f32, b as f32];
    }

    fn pixels(&self) -> &[u8] {
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
        // self.clear();
        self.clear_with_color(color.r() as f32, color.g() as f32, color.b() as f32);
        // self.fill(&color.into());
        Ok(())
    }
}

fn filter_background(canvas: &mut Canvas, canvas2: &mut Canvas) {
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

fn filter_foreground(canvas: &mut Canvas, canvas2: &mut Canvas) {
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

fn color_lightness(curr_pixel: [f32; 3], lightness: f32) -> Rgb {
    let my_rgb = Srgb::new(curr_pixel[0], curr_pixel[1], curr_pixel[2]);
    let my_lch = Lch::from_color(my_rgb);
    let mut my_hsl: Hsl = my_lch.into_color();
    my_hsl.lightness *= lightness;
    return Srgb::from_color(my_hsl);
}

fn filter_bright_foreground(canvas: &mut Canvas, canvas2: &mut Canvas) {
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let curr_pixel2 = canvas.get_pixel(x, y);
            let curr_pixel = canvas2.get_pixel(x, y);
            if curr_pixel2[0] != 0.0 && curr_pixel2[1] != 0.0 && curr_pixel2[2] != 0.0 {
                // canvas.set_pixel(x, y, curr_pixel[0], curr_pixel[1], curr_pixel[2]);
                // lighten?
                let my_rgb = color_lightness(curr_pixel, 2.0);
                canvas.set_pixel(x, y, my_rgb.red, my_rgb.green, my_rgb.blue);
            } else {
                // darken
                let my_rgb = color_lightness(curr_pixel, 0.1);
                canvas.set_pixel(x, y, my_rgb.red, my_rgb.green, my_rgb.blue);
            }
        }
    }
}

fn main() {
    let client = MatrixClient::new(MatrixClientSettings {
        addr: "tcp://localhost:42024".to_string(),
    });

    let mut canvas = Canvas::new(64, 32);
    let mut canvas2 = Canvas::new(64, 32);
    let mut canvas3 = Canvas::new(64, 32);
    let mut frame_timer = FrameTimer::new();
    let mut scene = WaveScene::new(&canvas3);
    let mut clock_scene: ClockScene = ClockScene::new(&canvas2);
    let mut plasma_scene: PlasmaScene = PlasmaScene {};

    loop {
        let tick = frame_timer.tick();

        scene.tick(&mut canvas3, &tick);
        plasma_scene.tick(&mut canvas, &tick);
        clock_scene.tick(&mut canvas2, &tick);
        // filter_background(&mut canvas, &mut canvas2);
        filter_bright_foreground(&mut canvas2, &mut canvas3);
        client.send_frame(canvas2.pixels());

        frame_timer.wait_for_next_frame();
    }
}
