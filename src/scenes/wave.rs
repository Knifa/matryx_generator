use rand::Rng;

use crate::{Canvas, FrameTick, Scene};

pub struct WaveScene {
    map: Vec<f32>,
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
        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let index = (y * canvas.width + x) as usize;
                let value = self.map[index];

                let r: f32 = value.powf(4.0 + (value * 0.5 * value.cos()));
                let g: f32 = value.powf(3.0 + (value * 0.5 * value.sin()));
                let b: f32 = value.powf(2.0 + (value * 0.5));

                canvas.set_pixel(x, y, r, g, b);
            }
        }
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

                map[i] = last_value * (0.96 + 0.02 * rng.gen::<f32>());
                if last_value <= (0.18 + 0.04 * rng.gen::<f32>()) {
                    let mut n = 0;

                    for u in -1..2 {
                        for v in -1..2 {
                            if u == 0 && v == 0 {
                                continue;
                            }

                            let x2 = ((x as i32 + u) % canvas.width as i32).abs() as u32;
                            let y2 = ((y as i32 + v) % canvas.height as i32).abs() as u32;

                            let i2 = (y2 * canvas.width + x2) as usize;
                            let last_value2 = last_map[i2];

                            if last_value2 > (0.5 + 0.04 * rng.gen::<f32>()) {
                                map[i] += last_value2 * (0.8 + 0.4 * rng.gen::<f32>());
                                n += 1;
                            }
                        }
                    }

                    if n > 0 {
                        map[i] /= n as f32;
                    }

                    if map[i] > 1.0 {
                        map[i] = 1.0;
                    }
                }
            }
        }

        self.map = map;
        self.draw_map(canvas);
    }
}
