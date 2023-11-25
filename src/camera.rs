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


use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{Dimensions, Point, Size},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::Rectangle,
    Pixel,
};

pub fn cam_thread_loop(hists_clone: Arc<AtomicU8>) {
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
    }
}
