use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::Drawable;

mod adc;
mod helper;
mod lcd;
mod timers;

pub use adc::AdcConfig;
pub use helper::*;
pub use lcd::IliError;
pub use timers::{BeatTimer, FrameTimer};

pub trait Lcd {
    type Error;
    fn clear(&mut self, color: Rgb565) -> Result<(), Self::Error>;
    fn draw<D: Drawable<Color = Rgb565>>(&mut self, drawable: D) -> Result<D::Output, Self::Error>;
    fn scroll(&mut self, num_of_lines: u16) -> Result<(), Self::Error>;
}
