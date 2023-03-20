use crate::{Canvas, FrameTick, Scene};
use chrono::Local;
use embedded_graphics::{
    mono_font::{ascii::FONT_4X6, MonoTextStyle},
    pixelcolor::Rgb888,
    prelude::*,
    text::Text,
    draw_target::DrawTarget,
    geometry::{Dimensions, Point, Size},
    primitives::Rectangle,
    Pixel,
};

extern crate chrono;


pub struct ClockScene {
    format_lines: Vec<String>,
}

impl ClockScene {
    pub fn new(canvas: &Canvas) -> Self {
        let mut format_lines = vec!["%H:%M".to_string()];
        // let path = Path::new("./foo/bar.txt");
        // let myFont = LedFont.new(path);
        // let mut rng = rand::thread_rng();

        // let mut map = vec![0.0_f32; (canvas.width * canvas.height) as usize];
        // for i in &mut map {
        //     *i = rng.gen();
        // }

        // let weights = gen_weights();

        ClockScene {
            // last_map: map.clone(),
            // map,
            // weights,
            format_lines,
        }
    }
}


impl Scene for ClockScene {
    fn tick(&mut self, canvas: &mut Canvas, tick: &FrameTick) {
        let t = tick.start.elapsed().as_secs_f32() * 0.5f32;
        let date = Local::now();
        let times=date.format("%H:%M:%S").to_string();
        // println!("{}", times);

        let text_style = MonoTextStyle::new(&FONT_4X6, Rgb888::new(0xff, 0xff, 0xff));
        // let eg_text = "EG+";
        // canvas.clear();
        Text::new(&times, Point::new(16, 16), text_style)
            .draw( canvas)
            .unwrap();
    }
}
