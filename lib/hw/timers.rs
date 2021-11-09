use stm32g0xx_hal::hal::timer::CountDown;
use stm32g0xx_hal::hal::PwmPin as PwmPinTrait;
use stm32g0xx_hal::rcc::Rcc;
use stm32g0xx_hal::stm32g0::stm32g070::{RCC, TIM1, TIM3, TIM6, TIM7};
use stm32g0xx_hal::time::{Hertz, MicroSecond};
use stm32g0xx_hal::timer::pins::TimerPin;
use stm32g0xx_hal::timer::pwm::{Pwm, PwmExt, PwmPin};
use stm32g0xx_hal::timer::{Channel1, Channel4, Timer, TimerExt};

pub struct FrameTimer {
    timer: Timer<TIM6>,
    freq: Hertz,
}

impl FrameTimer {
    pub fn new(pac_tim: TIM6, freq: Hertz, rcc: &mut Rcc) -> Self {
        FrameTimer {
            timer: pac_tim.timer(rcc),
            freq,
        }
    }

    pub fn start(&mut self) {
        self.timer.clear_irq();
        self.timer.listen();
        self.timer.start(self.freq);
    }

    pub fn unpend(&mut self) {
        self.timer.clear_irq();
    }
}

pub struct BeatTimer {
    timer: Timer<TIM7>,
    timeout: MicroSecond,
}

impl BeatTimer {
    pub fn new(pac_tim: TIM7, timeout: MicroSecond, rcc: &mut Rcc) -> Self {
        BeatTimer {
            timer: pac_tim.timer(rcc),
            timeout,
        }
    }

    pub fn start(&mut self) {
        self.timer.clear_irq();
        self.timer.listen();
        self.timer.start(self.timeout);
    }

    pub fn unpend(&mut self) {
        self.timer.clear_irq();
    }
}

pub struct BeatCounterTimer<I> {
    timer: TIM3,
    _input: I,
}

impl<I> BeatCounterTimer<I>
where
    I: TimerPin<TIM3, Channel = Channel1>,
{
    pub fn new(pac_timer: TIM3, input: I, rcc: &mut Rcc) -> Self {
        BeatCounterTimer::<I>::enable_clock_and_reset(rcc);
        input.setup();

        let mut counter = BeatCounterTimer {
            timer: pac_timer,
            _input: input,
        };
        counter.configure();
        counter
    }

    pub fn start(&mut self) {
        self.timer.cr1.modify(|_, w| w.cen().set_bit());
    }

    pub fn read(&self) -> u16 {
        self.timer.cnt.read().cnt_l().bits()
    }

    pub fn reset(&mut self) {
        self.timer.cnt.reset();
    }

    fn configure(&mut self) {
        // Divide input clock by 4 for tDTS
        self.timer.cr1.write(|w| unsafe { w.ckd().bits(0b10) });
        // Select input Tx -> CH1 Input
        self.timer
            .tisel
            .write(|w| unsafe { w.ti1sel().bits(0b0000) });
        self.timer.ccmr1_input().write(|w| unsafe {
            // IC1 -> T1
            w.cc1s().bits(0b01);
            // Filter tDTS * 4, 8 samples
            w.ic1f().bits(0b1111);
            // No prescaler
            w.ic1psc().bits(0b00)
        });
        self.timer.ccer.write(|w| {
            // Non-inverted rising edge
            w.cc1p().clear_bit();
            w.cc1np().clear_bit()
        });
        self.timer.smcr.write(|w| unsafe {
            // External clock 1
            w.sms().bits(0b111);
            // T1FP1 as trigger
            w.ts().bits(0b101)
        });

        // Set prescaler as 0
        self.timer.psc.write(|w| unsafe { w.psc().bits(0) });
        // Set ARR to max value
        self.timer
            .arr
            .write(|w| unsafe { w.arr_l().bits(u16::max_value()) });

        // Trigger update event to load the registers
        self.timer.cr1.modify(|_, w| w.urs().set_bit());
        self.timer.egr.write(|w| w.ug().set_bit());
        self.timer.cr1.modify(|_, w| w.urs().clear_bit());
    }

    fn enable_clock_and_reset(_: &mut Rcc) {
        let rcc = unsafe { &(*RCC::ptr()) };
        rcc.apbenr1.modify(|_, w| w.tim3en().set_bit());
        rcc.apbrstr1.modify(|_, w| w.tim3rst().set_bit());
        rcc.apbrstr1.modify(|_, w| w.tim3rst().clear_bit());
    }
}

struct UnusedPin;

impl TimerPin<TIM1> for UnusedPin {
    type Channel = Channel4;

    fn setup(&self) {
        // Do nothing
    }

    fn release(self) -> Self {
        self
    }
}

pub struct SampleTimer {
    _timer: Pwm<TIM1>,
    trig: PwmPin<TIM1, Channel4>,
}

impl SampleTimer {
    pub fn new(pac_timer: TIM1, freq: Hertz, rcc: &mut Rcc) -> Self {
        let timer = pac_timer.pwm(freq, rcc);
        let trig = timer.bind_pin(UnusedPin);
        SampleTimer {
            _timer: timer,
            trig,
        }
    }

    pub fn start(&mut self) {
        self.trig.set_duty(self.trig.get_max_duty() / 2);
        self.trig.enable();
    }
}
