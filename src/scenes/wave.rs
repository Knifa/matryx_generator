use palette::{FromColor, Oklch, Srgb};
use rand::Rng;

use crate::{Canvas, FrameTick, Scene};

const SEARCH_RADIUS: i32 = 2;
const KERNEL_SIZE: usize = (SEARCH_RADIUS * 2 + 1) as usize;

type Kernel = [[f32; KERNEL_SIZE]; KERNEL_SIZE];

pub struct WaveScene {
    map: Vec<f32>,
    last_map: Vec<f32>,
    weights: Kernel,
}

impl WaveScene {
    pub fn new(canvas: &Canvas) -> Self {
        let mut rng = rand::thread_rng();

        let mut map = vec![0.0_f32; (canvas.width * canvas.height) as usize];
        for i in &mut map {
            *i = rng.gen();
        }

        let weights = gen_weights();

        WaveScene {
            last_map: map.clone(),
            map,
            weights,
        }
    }

    fn draw_map(&self, canvas: &mut Canvas, t: f32) {
        //let median_map = median_filter(&self.map, canvas);
        let map = &self.map;

        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let index = (y * canvas.width + x) as usize;
                let value = map[index].powf(2.0);

                let hsv = Oklch::new(value.powf(1.0), 0.1, (value + t * 0.1) * 360.0);
                let rgb = Srgb::from_color(hsv);

                canvas.set_pixel(x, y, rgb.red, rgb.green, rgb.blue);
            }
        }
    }
}

fn gen_weights() -> Kernel {
    let mut weights = [[0.0_f32; KERNEL_SIZE]; KERNEL_SIZE];
    for y in -SEARCH_RADIUS..SEARCH_RADIUS + 1 {
        for x in -SEARCH_RADIUS..SEARCH_RADIUS + 1 {
            let ix = (x + SEARCH_RADIUS) as usize;
            let iy = (y + SEARCH_RADIUS) as usize;

            let dist = (x * x + y * y) as f32;
            let weight = (1.0 / dist).powf(0.1);

            weights[iy][ix] = weight;
        }
    }
    weights
}

fn grow_step(x: u32, y: u32, map: &Vec<f32>, canvas: &Canvas, weights: &Kernel) -> f32 {
    let mut rng = rand::thread_rng();

    let i = (y * canvas.width + x) as usize;
    let mut val = map[i];

    let mut n = 0.0;
    let mut c = 0.0;

    for u in -SEARCH_RADIUS..SEARCH_RADIUS + 1 {
        for v in -SEARCH_RADIUS..SEARCH_RADIUS + 1 {
            if u == 0 && v == 0 {
                continue;
            }

            let x2 = ((x as i32 + u) % canvas.width as i32).abs() as u32;
            let y2 = ((y as i32 + v) % canvas.height as i32).abs() as u32;
            let i2 = (y2 * canvas.width + x2) as usize;
            let last_value2 = map[i2];

            if last_value2 > rng.gen_range(0.4..0.6) {
                let weight = weights[(v + SEARCH_RADIUS) as usize][(u + SEARCH_RADIUS) as usize];

                c += last_value2 * rng.gen_range(0.9..1.1) * weight;
                n += weight;
            }
        }
    }

    val = (val + c) / n;
    val.clamp(0.0, 1.0)
}

fn median_filter(map: &Vec<f32>, canvas: &Canvas) -> Vec<f32> {
    const MEDIAN_WINDOW: i32 = 1;

    let mut filtered = vec![0.0; (canvas.width * canvas.height) as usize];
    let mut window = Vec::<f32>::new();

    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let i = (y * canvas.width + x) as usize;

            for u in -MEDIAN_WINDOW..MEDIAN_WINDOW + 1 {
                for v in -MEDIAN_WINDOW..MEDIAN_WINDOW + 1 {
                    let x2 = ((x as i32 + u) % canvas.width as i32).abs() as u32;
                    let y2 = ((y as i32 + v) % canvas.height as i32).abs() as u32;
                    let i2 = (y2 * canvas.width + x2) as usize;

                    let value = map[i2];
                    window.push(value);
                }
            }

            window.sort_by(|a, b| a.partial_cmp(b).unwrap());

            filtered[i] = window[window.len() / 2];

            window.clear();
        }
    }

    filtered
}

impl Scene for WaveScene {
    fn tick(&mut self, canvas: &mut Canvas, tick: &FrameTick) {
        let mut rng = rand::thread_rng();

        std::mem::swap(&mut self.last_map, &mut self.map);
        let last_map = &mut self.last_map;
        let map = &mut self.map;

        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let i = (y * canvas.width + x) as usize;
                let last_value = last_map[i];

                map[i] = last_value * (1.0 - (rng.gen_range(0.2..0.4) * tick.dt));

                if last_value <= rng.gen_range(0.1..0.35) {
                    map[i] = grow_step(x, y, &last_map, canvas, &self.weights);
                }

                map[i] = map[i].clamp(0.0, 1.0);
            }
        }

        self.draw_map(canvas, tick.t);
    }
}
