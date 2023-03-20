use crate::{Canvas, FrameTick, Scene};
use chrono::Local;
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_9X18_BOLD},
    pixelcolor::Rgb888,
    prelude::*,
    text::Text,
    geometry::{Point},
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
        //TODO: format lines
        let times=date.format("%H:%M").to_string();
        let text_style = MonoTextStyle::new(&FONT_9X18_BOLD, Rgb888::new(0, 0,0));
        // canvas.clear();
        // Text::new(&times, Point::new(9, 18), text_style)
        Text::new(&times, Point::new(10, 20), text_style)
            .draw( canvas)
            .unwrap();
    }
}
