use crate::{Canvas, FrameTick, Scene};
use chrono::Local;
use embedded_graphics::{
    geometry::Point,
    pixelcolor::{Rgb888},
    prelude::*,
    text::Text,
};
use u8g2_fonts::{fonts, U8g2TextStyle};

extern crate chrono;

pub struct ClockScene {
    // format_lines: Vec<String>,
}

impl ClockScene {
    pub fn new(canvas: &Canvas) -> Self {
        // let mut format_lines = vec!["%H:%M".to_string()];

        // ClockScene { format_lines }
        ClockScene {  }
    }
}

impl Scene for ClockScene {
    fn tick(&mut self, canvas: &mut Canvas, tick: &FrameTick) {
        // let t = tick.start.elapsed().as_secs_f32() * 0.5f32;
        let date = Local::now();
        //TODO: format lines
        canvas.clear();
        // let font = FontRenderer::new::<fonts::u8g2_font_haxrcorp4089_t_cyrillic>();
        // let text = "embedded-graphics";
        // font.render_aligned(
        //     text,
        //     canvas.bounding_box().center() + Point::new(0, 16),
        //     VerticalPosition::Baseline,
        //     HorizontalAlignment::Center,
        //     FontColor::Transparent(BinaryColor::On),
        //     canvas,
        // )
        // .unwrap();
        let times = date.format("%H:%M").to_string();
        // // let text_style = MonoTextStyle::new(
        //     &u8g2_font_haxrcorp4089_t_cyrillic,
        //     Rgb888::new(0xff, 0xff, 0xff),
        // );
        // canvas.clear();
        let character_style =
        U8g2TextStyle::new(fonts::u8g2_font_helvB14_tn, Rgb888::new(255, 255, 255));

        // Text::new(&times, Point::new(9, 18), text_style)
        Text::new(&times, Point::new(8, 22), character_style)
            .draw(canvas)
            .unwrap();
    }
}
