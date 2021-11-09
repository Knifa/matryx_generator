use std::ops::Div;

use palette::{FromColor, Gradient, Hsv, LinSrgb, Srgb};
use rand::Rng;

use crate::{Canvas, FrameTick, Scene};

pub struct WaveScene {
    map: Vec<f32>,
}

fn from_u8_color(r: u8, g: u8, b: u8) -> LinSrgb {
    LinSrgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

impl WaveScene {
    pub fn new(canvas: &Canvas) -> Self {
        let mut rng = rand::thread_rng();

        let mut map = vec![0.0_f32; (canvas.width * canvas.height) as usize];
        for i in &mut map {
            *i = rng.gen();
        }

        WaveScene { map }
    }

    fn draw_map(&self, canvas: &mut Canvas) {
        let gradient = Gradient::new(vec![
            from_u8_color(234, 196, 53),
            from_u8_color(120, 150, 149),
            from_u8_color(3, 206, 164),
            from_u8_color(255, 255, 255),
        ]);

        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let index = (y * canvas.width + x) as usize;
                let value = self.map[index].powf(1.0);

                //let hsv = Hsv::new((value + 0.7) * 360.0, 1.0, value.powf(2.0));
                //let rgb = Srgb::from_color(hsv);

                let gv = gradient.get(value);
                let mut hsv = Hsv::from_color(gv);
                hsv.value *= value.powf(2.0);

                let rgb = LinSrgb::from_color(hsv);
                let rgb = Srgb::from_linear(rgb);


                canvas.set_pixel(x, y, rgb.red, rgb.green, rgb.blue);
            }
        }
    }
}

fn grow_step(x: u32, y: u32, map: &Vec<f32>, canvas: &Canvas) -> f32 {
    const SEARCH_RADIUS: i32 = 4;
    let mut rng = rand::thread_rng();

    let i = (y * canvas.width + x) as usize;
    let mut val = map[i];

    let mut n = 0.0;
    let mut cum = 0.0;

    for u in -SEARCH_RADIUS..SEARCH_RADIUS {
        for v in -SEARCH_RADIUS..SEARCH_RADIUS {
            if u == 0 && v == 0 {
                continue;
            }

            let x2 = ((x as i32 + u) % canvas.width as i32).abs() as u32;
            let y2 = ((y as i32 + v) % canvas.height as i32).abs() as u32;

            let dist = (u * u + v * v) as f32;
            let weight = (1.0 / dist).powf(1.5);

            let i2 = (y2 * canvas.width + x2) as usize;
            let last_value2 = map[i2];

            if last_value2 > rng.gen_range(0.4..0.6) {
                cum += last_value2 * rng.gen_range(0.8..1.1) * weight;
                n += weight;
            }
        }
    }

    val = (val + cum) / n;
    val.clamp(0.0, 1.0)
}

fn ease_grow(x: f32) -> f32 {
    let mut rng = rand::thread_rng();
    let lim = rng.gen_range(0.2..0.3);

    if x <= lim {
        1.0
    } else {
        0.0
    }
}

impl Scene for WaveScene {
    fn tick(&mut self, canvas: &mut Canvas, _tick: &FrameTick) {
        let mut rng = rand::thread_rng();

        let last_map = self.map.clone();
        let mut map = vec![0.0; 64 * 32];

        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let i = (y * canvas.width + x) as usize;
                let last_value = last_map[i];

                map[i] = last_value * rng.gen_range(0.96..0.99);

                let e = ease_grow(last_value);
                map[i] = e * grow_step(x, y, &last_map, canvas) + (1.0 - e) * map[i];

                map[i] = map[i].clamp(0.0, 1.0);
            }
        }

        self.map = map;
        self.draw_map(canvas);
    }
}
