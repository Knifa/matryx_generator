mod scenes;

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

fn filter_bright_foreground(canvas: &mut Canvas, canvas2: &mut Canvas, lightness: f32) {
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

fn filter_bright_background(canvas: &mut Canvas, canvas2: &mut Canvas, lightness: f32) {
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

fn filter_darken(canvas: &mut Canvas, lightness: f32) {
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let curr_pixel = canvas.get_pixel(x, y);
            let my_rgb = color_lightness(curr_pixel, lightness);
            canvas.set_pixel(x, y, my_rgb.red, 0.0, 0.0);
        }
    }
}

fn filter_red(canvas: &mut Canvas) {
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let curr_pixel = canvas.get_pixel(x, y);
            canvas.set_pixel(x, y, curr_pixel[0], 0.0, 0.0);
        }
    }
}

fn filter_hue_shift(canvas: &mut Canvas, shift: f32) {
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

fn main() {
    let client = MatrixClient::new(MatrixClientSettings {
        addr: "tcp://localhost:42024".to_string(),
    });

    #[cfg(debug_assertions)]
    let _log2 = log2::open("matryx-debug.txt")
        .size(100 * 1024 * 1024)
        .rotate(2)
        .tee(true)
        .start();

    #[cfg(not(debug_assertions))]
    let _log2 = log2::open("matryx-release.txt")
        .size(100 * 1024 * 1024)
        .rotate(2)
        .tee(false)
        .start();

    warn!("33");
    // let mut canvas_plasma = Canvas::new(64, 32);
    let mut canvas_clock = Canvas::new(64, 32);
    let mut canvas_wave = Canvas::new(64, 32);
    let mut frame_timer = FrameTimer::new();
    let mut scene = WaveScene::new(&canvas_wave, 1.0);
    let mut clock_scene: ClockScene = ClockScene::new(&canvas_clock);
    // let mut plasma_scene: PlasmaScene = PlasmaScene::new(0.1);
    let hists = Arc::new(AtomicU8::new(100));
    let hists_clone = hists.clone();

    let mut handle_vec = vec![]; // JoinHandles will go in here

    let handle = thread::spawn(move || cam_thread_loop(hists_clone));
    handle_vec.push(handle); // save the handle so we can call join on it outside of the loop

    let mut shifter: f32 = -180.0;

    loop {
        let tick = frame_timer.tick();
        clock_scene.tick(&mut canvas_clock, &tick);
        //warn!("{0}", hists.load(Ordering::Acquire));
        if hists.load(Ordering::Acquire) <= 24 {
            filter_darken(&mut canvas_clock, 0.003922);
            // filter_red(&mut canvas_clock);
            client.send_brightness(100);
            client.send_frame(canvas_clock.pixels());
        } else {
            scene.tick(&mut canvas_wave, &tick);
            // plasma_scene.tick(&mut canvas_plasma, &tick);
            // let mut canvas4 = canvas_wave.clone();
            // filter_background(&mut canvas3, &mut canvas2);
            // filter_bright_foreground(&mut canvas4, &mut canvas_wave, 0.01);
            filter_bright_background(&mut canvas_wave, &mut canvas_clock, 0.1);
            if shifter == 180.0 {
                shifter = -180.0;
            } else {
                shifter = shifter + 1.0;
            }
            filter_hue_shift(&mut canvas_wave, shifter);
            client.send_brightness(10);
            client.send_frame(canvas_wave.pixels());
        }
        frame_timer.wait_for_next_frame();
    }
}

fn cam_thread_loop(hists_clone: Arc<AtomicU8>) {
    let mut attempt: i8 = 1;
    let mut cam_thread_ret = cam_thread(hists_clone.clone(), attempt);
    loop {
        match cam_thread_ret {
            Ok(v) => {
                warn!("unreachable: {v:?}");
            }
            Err(e) => {
                warn!("Camera Error: {e:?}");
                if attempt == std::i8::MAX {
                    attempt = 1;
                } else {
                    attempt = attempt + 1;
                }
                cam_thread_ret = cam_thread(hists_clone.clone(), attempt);
            }
        }
        sleep(Duration::from_secs(5));
        error!("cam thread loop slept")
    }
}

fn cam_thread(hists_clone: Arc<AtomicU8>, attempt: i8) -> Result<i32, i32> {
    error!("Camera time, Attempt: {}\n", attempt);
    // let mut dev = Device::new(2).unwrap();

    let mut dev = {
        let this = Device::new(0);
        match this {
            Ok(t) => t,
            Err(e) => {
                error!("Device missing: {}", e);
                return Err(-1);
            }
        }
    };

    // Let's say we want to explicitly request another format
    let mut format = {
        let this = dev.format();
        match this {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to read format: {}", e);
                return Err(-1);
            }
        }
    };
    error!("format set");
    // let mut format = dev.format().expect("Failed to read format");
    // let mut params = dev.params().expect("Failed to read params");

    // fmt.width = 1280;
    // fmt.height = 720;
    // fmt.fourcc = FourCC::new(b"YUYV");
    // try RGB3 first
    // warn!("Active format:\n{}", format);
    // warn!("Active parameters:\n{}", params);

    format.fourcc = FourCC::new(b"RGB3");
    // format = dev.set_format(&format).unwrap();
    format = {
        let this = dev.set_format(&format);
        match this {
            Ok(t) => t,
            Err(e) => {
                error!("set format {}", e);
                return Err(-1);
            }
        }
    };

    if format.fourcc != FourCC::new(b"RGB3") {
        // fallback to Motion-JPEG
        format.fourcc = FourCC::new(b"MJPG");
        // format = dev.set_format(&format).unwrap();
        format = {
            let this = dev.set_format(&format);
            match this {
                Ok(t) => t,
                Err(e) => {
                    error!("set format {}", e);
                    return Err(-1);
                }
            }
        };
    }

    error!("Active format:\n{}", format);
    // warn!("Active parameters:\n{}", params);

    // The actual format chosen by the device driver may differ from what we
    // requested! Print it out to get an idea of what is actually used now.

    // let controls = dev.query_controls().unwrap();

    // for control in controls {
    //     warn!("{}", control);
    // }

    // warn!("Available formats:");
    // for format in dev.enum_formats().unwrap() {
    //     warn!("  {} ({})", format.fourcc, format.description);

    //     for framesize in dev.enum_framesizes(format.fourcc).unwrap() {
    //         for discrete in framesize.size.to_discrete() {
    //             warn!("    Size: {}", discrete);

    //             for frameinterval in dev
    //                 .enum_frameintervals(framesize.fourcc, discrete.width, discrete.height)
    //                 .unwrap()
    //             {
    //                 warn!("      Interval:  {}", frameinterval);
    //             }
    //         }
    //     }
    //     warn!();
    // }

    // Now we'd like to capture some frames!
    // First, we need to create a stream to read buffers from. We choose a
    // mapped buffer stream, which uses mmap to directly access the device
    // frame buffer. No buffers are copied nor allocated, so this is actually
    // a zero-copy operation.

    // To achieve the best possible performance, you may want to use a
    // UserBufferStream instance, but this is not supported on all devices,
    // so we stick to the mapped case for this example.
    // Please refer to the rustdoc docs for a more detailed explanation about
    // buffer transfers.

    // Create the stream, which will internally 'allocate' (as in map) the
    // number of requested buffers for us.
    // let mut stream = MmapStream::with_buffers(&mut dev, Type::VideoCapture, 1)
    //     .expect("Failed to create buffer stream");

    error!("starting stream");
    let mut stream = {
        let this = UserptrStream::with_buffers(&mut dev, Type::VideoCapture, 1);
        match this {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to create buffer stream {}", e);
                return Err(-1);
            }
        }
    };
    error!("stream started");

    // At this point, the stream is ready and all buffers are setup.
    // We can now read frames (represented as buffers) by iterating through
    // the stream. Once an error condition occurs, the iterator will return
    // None.
    let count = 1;
    let frame_delay = time::Duration::from_millis(500);

    loop {
        let rstart = Instant::now();
        debug!("grab next image");
        let mut start = Instant::now();
        let _ = stream.next();
        let (buf, _) = {
            let this = stream.next();
            match this {
                Ok(t) => t,
                Err(e) => {
                    error!("Camera thread dead: {}", e);
                    return Err(-1);
                }
            }
        };
        let duration_us = start.elapsed().as_micros();
        debug!("next image grabbed {}", duration_us);
        start = Instant::now();
        let data = match &format.fourcc.repr {
            b"RGB3" => buf.to_vec(),
            b"MJPG" => {
                // Decode the JPEG frame to RGB
                let mut decoder = jpeg::Decoder::new(buf);
                decoder.decode().expect("failed to decode JPEG")
            }
            _ => {
                error!("invalid buffer pixelformat");
                return Err(-2);
            }
        };
        let duration_us = start.elapsed().as_micros();
        debug!("vectorized, {}", duration_us);
        start = Instant::now();
        let img: ImageBuffer<image::Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_raw(format.width, format.height, data).unwrap();
        let duration_us = start.elapsed().as_micros();
        debug!("wrapped to image buffer{}", duration_us);
        start = Instant::now();
        let luma = DynamicImage::ImageRgb8(img).into_luma8();
        let duration_us = start.elapsed().as_micros();
        debug!("luma'd {}", duration_us);
        start = Instant::now();
        let val = percentile(&luma, 90);
        let duration_us = start.elapsed().as_micros();
        debug!("percentile'd {}", duration_us);
        start = Instant::now();
        hists_clone.store(val, Ordering::Relaxed);
        let duration_us = start.elapsed().as_micros();
        debug!("stored {} and {}", val, duration_us);
        debug!("FPS1: {}", count as f64 / rstart.elapsed().as_secs_f64());
        thread::sleep(frame_delay);
        debug!("FPS2: {}", count as f64 / rstart.elapsed().as_secs_f64());
        // warn!(
        //     "Buffer size: {}, seq: {}, timestamp: {}",
        //     buf.len(),
        //     meta.sequence,
        //     meta.timestamp
        // );
        // let val = percentile(buf, 90);
        // hists_clone.store(val, Ordering::Relaxed);

        // To process the captured data, you can pass it somewhere else.
        // If you want to modify the data or extend its lifetime, you have to
        // copy it. This is a best-effort tradeoff solution that allows for
        // zero-copy readers while enforcing a full clone of the data for
        // writers.
    }

    // let cameras = query(ApiBackend::Auto).unwrap();
    // if cameras.len() > 0 {
    //     // request the absolute highest resolution CameraFormat that can be decoded to RGB.
    //     let requested: RequestedFormat =
    //         RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution);
    //     // make the camera
    //     let mut camera = match CallbackCamera::new(CameraIndex::Index(0), requested, move |buf| {
    //         let val = percentile(&buf.decode_image::<LumaFormat>().unwrap(), 90);
    //         hists_clone.store(val, Ordering::Relaxed);
    //     }) {
    //         Ok(val) => val,
    //         Err(err) => {
    //             error!("{}\n", err);
    //             return Err(-2);
    //         }
    //     };
    //     camera.open_stream().unwrap();
    //     sleep(Duration::from_secs(3)); // otherwise thread does not finish spawning and the method returns (??)
    //                                    // loop {
    //     error!("2");
    //     // }
    //     return Err(-3); // also returns if thread dies
    // } else {
    // }
}

// CommandsProper::ListDevices => {
//     let backend = native_api_backend().unwrap();
//     let devices = query(backend).unwrap();
//     warn!("There are {} available cameras.", devices.len());
//     for device in devices {
//         warn!("{device}");
//     }
// }
// CommandsProper::ListProperties { device, kind } => {
//     let index = match device.as_ref().unwrap_or(&IndexKind::Index(0)) {
//         IndexKind::String(s) => CameraIndex::String(s.clone()),
//         IndexKind::Index(i) => CameraIndex::Index(*i),
//     };
//     let mut camera = Camera::new(
//         index,
//         RequestedFormat::new::<RgbFormat>(RequestedFormatType::None),
//     )
//     .unwrap();
//     match kind {
//         PropertyKind::All => {
//             camera_print_controls(&camera);
//             camera_compatible_formats(&mut camera);
//         }
//         PropertyKind::Controls => {
//             camera_print_controls(&camera);
//         }
//         PropertyKind::CompatibleFormats => {
//             camera_compatible_formats(&mut camera);
//         }
//     }
