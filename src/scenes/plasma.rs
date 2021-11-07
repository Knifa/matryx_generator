use crate::{Canvas, FrameTick, Scene};

pub struct PlasmaScene {}

impl Scene for PlasmaScene {
    fn tick(&mut self, canvas: &mut Canvas, tick: &FrameTick) {
        let t = tick.start.elapsed().as_secs_f32() * 0.5f32;

        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let xp =
                    ((x as f32 / 128.0) - 0.5) * (5.0 + (t * 0.25).sin()) + (t * 0.25).sin() * 5.0;
                let yp =
                    ((y as f32 / 128.0) - 0.5) * (5.0 + (t * 0.25).sin()) + (t * 0.25).cos() * 5.0;

                let pixel = (((0.25 * t).sin() * xp + (0.29 * t).cos() * yp + t).sin()
                    + (((xp + (t * 0.25).sin() * 4.0).powf(2.0)
                        + (yp + (t * 0.43).cos() * 4.0).powf(2.0))
                    .sqrt()
                        + t)
                        .sin()
                    - (((xp + (t * 0.36).cos() * 6.0).powf(2.0)
                        + (yp + (t * 0.39).sin() * 5.3).powf(2.0))
                    .sqrt()
                        + t)
                        .cos())
                .sin();

                let u = ((9.0 * pixel + 0.5 * xp + t).cos() * 0.5 + 0.5).powf(2.0);
                let v = ((9.0 * pixel + 0.5 * yp + t).sin() * 0.5 + 0.5).powf(2.0);

                canvas.set_pixel(x, y, u, v, (u + v) / 2.0)
            }
        }
    }
}
