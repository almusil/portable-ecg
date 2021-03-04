use core::convert::Infallible;
use display_interface_parallel_gpio::WriteOnlyDataCommand;
use embedded_graphics::drawable::Drawable;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::DrawTarget;
use ili9341::{DisplaySize320x480, Error, Ili9341, Orientation, Scroller};
use stm32g0xx_hal::hal::blocking::delay::DelayMs;
use stm32g0xx_hal::hal::digital::v2::OutputPin;

use crate::hw::Lcd;

#[derive(Debug)]
pub struct IliError(pub Error<Infallible>);

pub struct IliLcd<I, R> {
    ili: Ili9341<I, R>,
    scroller: Scroller,
}

impl<I, R> IliLcd<I, R>
where
    I: WriteOnlyDataCommand,
    R: OutputPin<Error = Infallible>,
{
    pub fn new<D>(
        interface: I,
        reset: R,
        scoller_offset: (u16, u16),
        delay: &mut D,
    ) -> Result<Self, IliError>
    where
        D: DelayMs<u16>,
    {
        let mut ili = Ili9341::new(
            interface,
            reset,
            delay,
            Orientation::Landscape,
            DisplaySize320x480,
        )
        .map_err(IliError)?;
        let scroller = ili
            .configure_vertical_scroll(scoller_offset.0, scoller_offset.1)
            .map_err(IliError)?;

        Ok(IliLcd { ili, scroller })
    }
}

impl<I, R> Lcd for IliLcd<I, R>
where
    I: WriteOnlyDataCommand,
    R: OutputPin<Error = Infallible>,
{
    type Error = IliError;

    fn clear(&mut self, color: Rgb565) -> Result<(), Self::Error> {
        self.ili.clear(color).map_err(IliError)
    }

    fn draw<D: Drawable<Rgb565>>(&mut self, drawable: D) -> Result<(), Self::Error> {
        drawable.draw(&mut self.ili).map_err(IliError)
    }

    fn scroll(&mut self, num_of_lines: u16) -> Result<(), Self::Error> {
        self.ili
            .scroll_vertically(&mut self.scroller, num_of_lines)
            .map_err(IliError)
    }
}
