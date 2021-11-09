use cortex_m::peripheral::SYST;
use display_interface_parallel_gpio::PGPIO8BitInterface;
use stm32g0xx_hal::dma::C1;
use stm32g0xx_hal::gpio::gpioa::{PA0, PA4, PA5, PA6};
use stm32g0xx_hal::gpio::gpiob::{PB0, PB1, PB2, PB3, PB4, PB5, PB6, PB7, PB8, PB9};
use stm32g0xx_hal::gpio::{Analog, DefaultMode, Output, PushPull};
use stm32g0xx_hal::prelude::OutputPin;
use stm32g0xx_hal::rcc::{Config, PllConfig, Rcc, RccExt};
use stm32g0xx_hal::stm32g0::stm32g070::RCC;
use stm32g0xx_hal::timer::delay::Delay;

use crate::hw::adc::{Adc as HwAdc, Calibration};
use crate::hw::lcd::{IliError, IliLcd};
use crate::hw::timers::BeatCounterTimer;

pub fn init_clock(pac_rcc: RCC) -> Rcc {
    // ((16 MHz / 4) * 32) / 2 = 64 MHz
    let pll_config = PllConfig::with_hsi(4, 32, 2);
    pac_rcc.freeze(Config::pll().pll_cfg(pll_config))
}

// PB0 - LCD_D0
type LcdD0 = PB0<Output<PushPull>>;
// PB1 - LCD_D1
type LcdD1 = PB1<Output<PushPull>>;
// PB2 - LCD_D2
type LcdD2 = PB2<Output<PushPull>>;
// PB3 - LCD_D3
type LcdD3 = PB3<Output<PushPull>>;
// PB4 - LCD_D4
type LcdD4 = PB4<Output<PushPull>>;
// PB5 - LCD_D5
type LcdD5 = PB5<Output<PushPull>>;
// PB6 - LCD_D6
type LcdD6 = PB6<Output<PushPull>>;
// PB7 - LCD_D7
type LcdD7 = PB7<Output<PushPull>>;
// PB8 - LCD_DC (Command[Low]/Data[High])
type LcdDC = PB8<Output<PushPull>>;
// PB9 - LCD_WR (Write signal)
type LcdWR = PB9<Output<PushPull>>;
// ADC DMA channel
type DmaChannel = C1;
// PA0 - ADC ECG input channel
type InputChannel = PA0<Analog>;
// PA6 - ECG beat counter input
type CounterInput = PA6<DefaultMode>;

// RESERVED for future use
// PA1 - Comparator threshold - ADC input
// PA2 - USART2_TX
// PA3 - USART2_RX

// PA4 - LCD_RST (Reset)
pub type LcdRst = PA4<Output<PushPull>>;
// PA5 - LCD_RD (Read signal)
pub type LcdRD = PA5<Output<PushPull>>;

pub type Adc = HwAdc<InputChannel, DmaChannel>;
pub type BeatCounter = BeatCounterTimer<CounterInput>;
pub type LcdInterface =
    PGPIO8BitInterface<LcdD0, LcdD1, LcdD2, LcdD3, LcdD4, LcdD5, LcdD6, LcdD7, LcdDC, LcdWR>;
pub type HwLcd = IliLcd<LcdInterface, LcdRst>;

pub fn init_lcd(
    interface: LcdInterface,
    lcd_rst: LcdRst,
    lcd_rd: LcdRD,
    scroller_offset: (u16, u16),
    delay: &mut Delay<SYST>,
) -> Result<HwLcd, IliError> {
    let mut lcd_rd = lcd_rd;
    lcd_rd.set_high().unwrap();
    IliLcd::new(interface, lcd_rst, scroller_offset, delay)
}

pub fn get_calibration() -> u16 {
    Calibration.vref_int.read()
}
