use core::fmt::Write;
use embedded_graphics::fonts::{Font12x16, Text};
use embedded_graphics::pixelcolor::{Rgb565, RgbColor};
use embedded_graphics::prelude::{Point, Primitive};
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::style::{PrimitiveStyle, PrimitiveStyleBuilder, TextStyle};
use heapless::spsc::Consumer;
use heapless::spsc::Queue;
use heapless::String;

use crate::error::{Error, Result};
use crate::hw::Lcd;

const SAMPLE_MAX: usize = 3450;
const SAMPLE_MIN: usize = 0;

pub struct Display<'a, LCD, LCDER, const LEN: usize>
where
    LCD: Lcd<Error = LCDER>,
{
    current_data: Queue<Data, 512>,
    buffer: Consumer<'a, u16, LEN>,
    horizontal_position: u16,
    last_sample: u16,
    last_bpm: u16,
    lcd: LCD,
}

impl<'a, LCD, LCDER, const LEN: usize> Display<'a, LCD, LCDER, LEN>
where
    LCD: Lcd<Error = LCDER>,
{
    pub fn new(lcd: LCD, buffer: Consumer<'a, u16, LEN>) -> Result<Self, LCDER> {
        let mut display = Display {
            current_data: Queue::new(),
            buffer,
            horizontal_position: (Frame::WIDTH - 1) as u16,
            last_sample: Display::<'a, LCD, LCDER, LEN>::map_sample(0),
            last_bpm: 0,
            lcd,
        };
        display.init()?;
        Ok(display)
    }

    pub fn frame(&mut self) -> Result<(), LCDER> {
        let len = self.buffer.len();
        for _ in 0..len {
            let sample = self.buffer.dequeue().ok_or(Error::Queue)?;
            // Scroll
            self.scroll()?;
            // Remove old data
            let data_to_remove = self.current_data.dequeue().ok_or(Error::Queue)?;
            self.draw_single(&data_to_remove, Color::BACKGROUND)?;
            // Draw current data
            let mapped_sample = Display::<'a, LCD, LCDER, LEN>::map_sample(sample);
            let data_to_add = (self.last_sample, mapped_sample).into();
            self.draw_single(&data_to_add, Color::DATA)?;
            self.current_data
                .enqueue(data_to_add)
                .map_err(|_| Error::Queue)?;
            // Save mapped as last
            self.last_sample = mapped_sample;
        }

        Ok(())
    }

    pub fn update_bpm(&mut self, bpm: u16) -> Result<(), LCDER> {
        if bpm != self.last_bpm {
            self.draw_bpm_value(self.last_bpm, Color::BACKGROUND)?;
            self.draw_bpm_value(bpm, Color::BPM_TEXT)?;
            self.last_bpm = bpm;
        }
        Ok(())
    }

    fn draw_bpm_value(&mut self, bpm: u16, color: Rgb565) -> Result<(), LCDER> {
        let mut buffer = String::<8>::new();
        write!(&mut buffer, "{:>3}", bpm).map_err(|_| Error::BufferWrite)?;
        let bpm_val = Text::new(&buffer, DataColumn::TEXT_BPM_VAL_POSITION)
            .into_styled(TextStyle::new(Font12x16, color));
        self.lcd.draw(&bpm_val).map_err(Error::Lcd)?;
        Ok(())
    }

    fn draw_single(&mut self, data: &Data, color: Rgb565) -> Result<(), LCDER> {
        let x = (Frame::TOP_LEFT.x as u16 + self.horizontal_position) as i32;
        let y = (Frame::BOTTOM_RIGHT.y as u16 - data.y) as i32;
        let rect = Rectangle::new(Point::new(x, y - data.height as i32), Point::new(x, y))
            .into_styled(PrimitiveStyle::with_fill(color));
        self.lcd.draw(&rect).map_err(Error::Lcd)
    }

    fn scroll(&mut self) -> Result<(), LCDER> {
        self.lcd.scroll(1).map_err(Error::Lcd)?;
        self.horizontal_position += 1;
        if self.horizontal_position >= Frame::WIDTH as u16 {
            self.horizontal_position = 0;
        }
        Ok(())
    }

    fn init(&mut self) -> Result<(), LCDER> {
        self.lcd.clear(Color::BACKGROUND).map_err(Error::Lcd)?;
        self.init_frame()?;
        self.init_data_column()?;
        self.init_data()?;
        Ok(())
    }

    fn init_frame(&mut self) -> Result<(), LCDER> {
        let top_left = Point::new(
            Frame::TOP_LEFT.x - Frame::BORDER_WIDTH,
            Frame::TOP_LEFT.y - Frame::BORDER_WIDTH,
        );
        let bottom_right = Point::new(
            Frame::BOTTOM_RIGHT.x + Frame::BORDER_WIDTH,
            Frame::BOTTOM_RIGHT.y + Frame::BORDER_WIDTH,
        );
        let border = Rectangle::new(top_left, bottom_right).into_styled(
            PrimitiveStyleBuilder::new()
                .stroke_width(Frame::BORDER_WIDTH as u32)
                .stroke_color(Color::FRAME_BORDER)
                .fill_color(Color::BACKGROUND)
                .build(),
        );
        self.lcd.draw(&border).map_err(Error::Lcd)?;
        Ok(())
    }

    fn init_data_column(&mut self) -> Result<(), LCDER> {
        let bpm = Text::new("BPM", DataColumn::TEXT_BPM_POSITION)
            .into_styled(TextStyle::new(Font12x16, Color::BPM_TEXT));
        self.draw_bpm_value(self.last_bpm, Color::BPM_TEXT)?;
        self.lcd.draw(&bpm).map_err(Error::Lcd)?;
        Ok(())
    }

    fn init_data(&mut self) -> Result<(), LCDER> {
        let zero = Display::<'a, LCD, LCDER, LEN>::map_sample(0);
        let data = (zero, zero).into();
        for _ in 0..Frame::WIDTH {
            self.draw_single(&data, Color::DATA)?;
            self.scroll()?;
            self.current_data.enqueue(data).map_err(|_| Error::Queue)?;
        }
        Ok(())
    }

    fn map_sample(sample: u16) -> u16 {
        map(
            sample as u32,
            SAMPLE_MIN as u32,
            SAMPLE_MAX as u32,
            0,
            Frame::HEIGHT as u32 - 1,
        ) as u16
    }
}

struct Dimension;

impl Dimension {
    const WIDTH: i32 = 480;
    const HEIGHT: i32 = 320;
}

pub(crate) struct Offset;

impl Offset {
    const BOTTOM: i32 = 10;
    const TOP: i32 = 10;
    pub(crate) const LEFT: i32 = 10;
    pub(crate) const RIGHT: i32 = 50;
}

struct Frame;

impl Frame {
    const BORDER_WIDTH: i32 = 2;

    const TOP_LEFT: Point = Point::new(Offset::LEFT, Offset::TOP);
    const BOTTOM_RIGHT: Point = Point::new(
        Dimension::WIDTH - Offset::RIGHT - 1,
        Dimension::HEIGHT - Offset::BOTTOM - 1,
    );

    const WIDTH: i32 = Dimension::WIDTH - Offset::LEFT - Offset::RIGHT;
    const HEIGHT: i32 = Dimension::HEIGHT - Offset::TOP - Offset::BOTTOM;
}

struct DataColumn;

impl DataColumn {
    const TEXT_WIDTH: i32 = 12 * 3;
    const TEXT_HEIGHT: i32 = 16;
    const TEXT_SPACING: i32 = 5;
    const TEXT_BPM_POSITION: Point = Point::new(
        Frame::BOTTOM_RIGHT.x
            + Frame::BORDER_WIDTH
            + (Offset::RIGHT - Frame::BORDER_WIDTH - DataColumn::TEXT_WIDTH) / 2,
        Frame::TOP_LEFT.y + DataColumn::TEXT_SPACING,
    );
    const TEXT_BPM_VAL_POSITION: Point = Point::new(
        DataColumn::TEXT_BPM_POSITION.x,
        DataColumn::TEXT_BPM_POSITION.y + DataColumn::TEXT_HEIGHT + DataColumn::TEXT_SPACING,
    );
}

struct Color;

impl Color {
    const BACKGROUND: Rgb565 = Rgb565::BLACK;
    const FRAME_BORDER: Rgb565 = Rgb565::WHITE;
    const DATA: Rgb565 = Rgb565::YELLOW;
    const BPM_TEXT: Rgb565 = Rgb565::RED;
}

#[derive(Copy, Clone)]
struct Data {
    pub(crate) y: u16,
    pub(crate) height: u16,
}

impl Data {
    fn new(y: u16, height: u16) -> Self {
        Data { y, height }
    }
}

impl From<(u16, u16)> for Data {
    fn from(samples: (u16, u16)) -> Self {
        let (y, height) = if samples.0 < samples.1 {
            (samples.0, samples.1 - samples.0)
        } else {
            (samples.1, samples.0 - samples.1)
        };
        Data::new(y, height)
    }
}

fn map(to_map: u32, in_min: u32, in_max: u32, out_min: u32, out_max: u32) -> u32 {
    (to_map - in_min) * (out_max - out_min) / (in_max - in_min) + out_min
}
