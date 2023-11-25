mod camera;
mod canvas;
mod scenes;
use crate::{canvas::{Canvas, filter_bright_background, filter_hue_shift}, camera::cam_thread_loop};

use image::{DynamicImage, ImageBuffer};
use imageproc::stats::percentile;
use led_matrix_zmq::client::{MatrixClient, MatrixClientSettings};

use jpeg_decoder as jpeg;
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

use scenes::{ClockScene, WaveScene};

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

const CAMERA_ON: bool = true;
const SHIFTER_START: f32 = -180.0;

fn main() {
    let client = MatrixClient::new(MatrixClientSettings {
        addrs: vec!["tcp://localhost:42024".to_string()],
    });

    #[cfg(debug_assertions)]
    let _log2 = log2::open("matryx-debug.txt")
        .size(100 * 1024 * 1024)
        .rotate(2)
        .tee(true)
        .level("trace")
        .start();

    #[cfg(not(debug_assertions))]
    let _log2 = log2::open("matryx-release.txt")
        .size(100 * 1024 * 1024)
        .rotate(2)
        .tee(false)
        .level("warn")
        .start();

    warn!("Matryx V4");
    let mut canvas_clock = Canvas::new(64, 32);
    let mut canvas_wave = Canvas::new(64, 32);
    let mut frame_timer = FrameTimer::new();
    let mut scene = WaveScene::new(&canvas_wave, 1.0);
    let mut clock_scene: ClockScene = ClockScene::new(&canvas_clock);
    let hists = Arc::new(AtomicU8::new(100));
    let hists_clone = hists.clone();

    if CAMERA_ON {
        let mut handle_vec = vec![]; // JoinHandles will go in here
        let handle = thread::spawn(move || cam_thread_loop(hists_clone));
        handle_vec.push(handle); // save the handle so we can call join on it outside of the loop
    }

    let mut shifter: f32 = SHIFTER_START;

    loop {
        let tick = frame_timer.tick();
        clock_scene.tick(&mut canvas_clock, &tick);
        debug!("camera light reading: {0}", hists.load(Ordering::Acquire));
        if hists.load(Ordering::Acquire) <= 24 {
            // filter_darken(&mut canvas_clock, 0.003922);
            // filter_red(&mut canvas_clock);
            client.send_brightness(10);
            client.send_frame(canvas_clock.pixels());
        } else {
            scene.tick(&mut canvas_wave, &tick);
            // plasma_scene.tick(&mut canvas_plasma, &tick);
            // let mut canvas4 = canvas_wave.clone();
            // filter_background(&mut canvas3, &mut canvas2);
            // filter_bright_foreground(&mut canvas4, &mut canvas_wave, 0.01);
            filter_bright_background(&mut canvas_wave, &mut canvas_clock, 0.1);
            if shifter == (SHIFTER_START * (-1.0)) {
                shifter = SHIFTER_START;
            } else {
                shifter = shifter + 1.0;
            }
            filter_hue_shift(&mut canvas_wave, shifter);
            client.send_brightness(100);
            client.send_frame(canvas_wave.pixels());
        }
        frame_timer.wait_for_next_frame();
    }
}
