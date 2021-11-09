mod scenes;

use led_matrix_zmq::client::{MatrixClient, MatrixClientSettings};
use std::time;

use scenes::{WaveScene};

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

    fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

fn main() {
    let client = MatrixClient::new(MatrixClientSettings {
        addr: "tcp://localhost:42024".to_string(),
    });

    let mut canvas = Canvas::new(64, 32);
    let mut frame_timer = FrameTimer::new();
    let mut scene = WaveScene::new(&canvas);

    loop {
        let tick = frame_timer.tick();

        scene.tick(&mut canvas, &tick);
        client.send_frame(canvas.pixels());

        frame_timer.wait_for_next_frame();
    }
}
