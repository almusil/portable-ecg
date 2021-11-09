use core::ops::Deref;
use stm32g0xx_hal::analog::adc::{Adc as HalAdc, VRef};
use stm32g0xx_hal::dma::{Channel as DmaChannel, Direction, Event, Priority, WordSize};
use stm32g0xx_hal::hal::adc::Channel as AdcChannel;
use stm32g0xx_hal::hal::blocking::delay::DelayUs;
use stm32g0xx_hal::rcc::Rcc;
use stm32g0xx_hal::stm32g0::stm32g070::{ADC, RCC, SYST, TIM1};
use stm32g0xx_hal::time::Hertz;
use stm32g0xx_hal::timer::delay::Delay;
use volatile_register::RO;

use crate::hw::timers::SampleTimer;
use crate::Buffer;

pub struct AdcConfig<I, C> {
    input: I,
    dma_channel: C,
    frequency: Hertz,
}

impl<I, C> AdcConfig<I, C>
where
    I: AdcChannel<HalAdc, ID = u8>,
    C: DmaChannel,
{
    pub fn new(input: I, dma_channel: C, frequency: Hertz) -> Self {
        AdcConfig {
            input,
            dma_channel,
            frequency,
        }
    }
}

pub struct Adc<I, C> {
    adc: InnerAdc<I>,
    dma: Dma<C>,
    trig: SampleTimer,
}

impl<I, C> Adc<I, C>
where
    I: AdcChannel<HalAdc, ID = u8>,
    C: DmaChannel,
{
    pub fn new(
        pac_adc: ADC,
        pac_timer: TIM1,
        buffer: &mut Buffer,
        config: AdcConfig<I, C>,
        rcc: &mut Rcc,
        delay: &mut Delay<SYST>,
    ) -> Self {
        let adc = InnerAdc::new(pac_adc, config.input, rcc, delay);
        let memory_addr = buffer.as_ptr() as u32;
        let dma = Dma::new(
            config.dma_channel,
            InnerAdc::<I>::get_dma_address(),
            memory_addr,
            buffer.len() as u16,
        );
        let trig = SampleTimer::new(pac_timer, config.frequency, rcc);
        Adc { adc, dma, trig }
    }

    pub fn start(&mut self) {
        self.adc.start();
        self.dma.start();
        self.trig.start();
    }

    pub fn unpend(&mut self) {
        self.dma.unpend();
    }
}

struct Dma<C> {
    channel: C,
}

impl<C> Dma<C>
where
    C: DmaChannel,
{
    pub fn new(channel: C, peripheral_addr: u32, memory_addr: u32, len: u16) -> Self {
        let mut dma = Dma { channel };
        dma.configure(peripheral_addr, memory_addr, len);
        dma
    }

    pub fn start(&mut self) {
        self.channel.clear_event(Event::HalfTransfer);
        self.channel.listen(Event::HalfTransfer);
        self.channel.enable();
    }

    pub fn unpend(&mut self) {
        self.channel.clear_event(Event::HalfTransfer);
    }

    fn configure(&mut self, peripheral_addr: u32, memory_addr: u32, len: u16) {
        self.channel.set_priority_level(Priority::VeryHigh);
        self.channel.set_word_size(WordSize::BITS16);
        self.channel.set_direction(Direction::FromPeripheral);
        self.channel.set_peripheral_address(peripheral_addr, false);
        self.channel.set_memory_address(memory_addr, true);
        self.channel.set_transfer_length(len);
        self.channel.set_circular_mode(true);
    }
}

struct InnerAdc<I> {
    adc: ADC,
    _input: I,
}

// FIXME Move this in some fashionable way upstream

impl<I> InnerAdc<I>
where
    I: AdcChannel<HalAdc, ID = u8>,
{
    pub fn new<D: DelayUs<u8>>(pac_adc: ADC, input: I, rcc: &mut Rcc, delay: &mut D) -> Self {
        InnerAdc::<I>::enable_clock_and_reset(rcc);
        let mut adc = InnerAdc {
            adc: pac_adc,
            _input: input,
        };
        adc.disable();
        adc.enable_vreg(delay);
        adc.calibrate();
        adc.enable();
        adc.configure();
        adc
    }

    pub fn start(&mut self) {
        self.adc.isr.write(|w| {
            w.eoc().set_bit();
            w.eos().set_bit()
        });
        self.adc.cr.modify(|_, w| w.adstart().set_bit());
    }

    pub fn get_dma_address() -> u32 {
        unsafe { &(*ADC::ptr()).dr as *const _ as u32 }
    }

    fn configure(&mut self) {
        self.adc.cfgr1.write(|w| unsafe {
            // External trigger rising edge
            w.exten().bits(0b01);
            // External trigger 1
            w.extsel().bits(0b001);
            // Right alignment
            w.align().clear_bit();
            // 12-bit resolution
            w.res().bits(0b00);
            // Circular DMA
            w.dmacfg().set_bit();
            // Enable DMA requests
            w.dmaen().set_bit()
        });
        // Enable Vref
        self.adc.ccr.write(|w| w.vrefen().set_bit());
        // 160.5 cycles for the best precision
        self.adc.smpr.write(|w| unsafe { w.smp1().bits(0b111) });
        // Select input channel and Vref
        self.adc
            .chselr()
            .write(|w| unsafe { w.chsel().bits(1 << VRef::channel() | 1 << I::channel()) });
    }

    fn enable_clock_and_reset(_: &mut Rcc) {
        let rcc = unsafe { &(*RCC::ptr()) };
        rcc.apbenr2.modify(|_, w| w.adcen().set_bit());
        rcc.apbrstr2.modify(|_, w| w.adcrst().set_bit());
        rcc.apbrstr2.modify(|_, w| w.adcrst().clear_bit());
    }

    fn enable_vreg<D: DelayUs<u8>>(&mut self, delay: &mut D) {
        self.adc.cr.modify(|_, w| w.advregen().set_bit());
        // Max starting time declared by stm32g070 datasheet is 20 us
        delay.delay_us(20);
    }

    fn enable(&mut self) {
        self.adc.isr.write(|w| w.adrdy().set_bit());
        self.adc.cr.modify(|_, w| w.aden().set_bit());
        while self.adc.isr.read().adrdy().bit_is_clear() {}
    }

    fn disable(&mut self) {
        let cr = self.adc.cr.read();
        if cr.aden().bit_is_clear() {
            return;
        }
        if cr.adstart().bit_is_set() {
            self.adc.cr.modify(|_, w| w.adstp().set_bit());
        }
        self.adc.cr.modify(|_, w| w.addis().set_bit());
        while self.adc.cr.read().aden().bit_is_set() {}
        self.adc.isr.write(|w| w.adrdy().set_bit());
    }

    fn calibrate(&mut self) {
        self.adc.cr.modify(|_, w| w.adcal().set_bit());
        while self.adc.isr.read().eocal().bit_is_clear() {}
        self.adc.isr.write(|w| w.eocal().set_bit());
    }
}

pub struct Calibration;

impl Calibration {
    #[inline(always)]
    pub fn ptr() -> *const CalibrationRegBlock {
        0x1fff_75aa as *const _
    }
}

impl Deref for Calibration {
    type Target = CalibrationRegBlock;

    fn deref(&self) -> &Self::Target {
        unsafe { &*Self::ptr() }
    }
}

#[repr(C)]
pub struct CalibrationRegBlock {
    pub vref_int: RO<u16>,
}
