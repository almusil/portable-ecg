#![no_main]
#![no_std]

use lib as _;

use cortex_m::singleton;
use heapless::spsc::Queue;
use lib::display::Display;
use lib::hw::{
    get_calibration, init_clock, init_lcd, Adc, AdcConfig, BeatCounter, BeatTimer, FrameTimer,
    HwLcd, IliError, LcdInterface,
};
use lib::sampler::Sampler;
use lib::{BOTTOM_SCROLL_OFFSET, TOP_SCROLL_OFFSET};
use rtic::app;
use stm32g0xx_hal::delay::DelayExt;
use stm32g0xx_hal::dma::DmaExt;
use stm32g0xx_hal::dmamux::DmaMuxIndex;
use stm32g0xx_hal::gpio::{GpioExt, Speed};
use stm32g0xx_hal::time::U32Ext;

#[app(device = stm32g0xx_hal::stm32, peripherals = true)]
const APP: () = {
    struct Resources {
        display: Display<'static, HwLcd, IliError, 64>,
        sampler: Sampler<'static, 64>,
        frame_timer: FrameTimer,
        adc: Adc,
        beat_timer: BeatTimer,
        beat_counter: BeatCounter,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        let core: rtic::export::Peripherals = cx.core;
        let device: stm32g0xx_hal::stm32::Peripherals = cx.device;

        // Buffers
        let queue: &'static mut Queue<_, 64> = singleton!(: Queue<u16, 64> = Queue::new()).unwrap();
        let dma_buffer: &'static mut [u16; 4] = singleton!(: [u16; 4] = [0; 4]).unwrap();
        let (producer, consumer) = queue.split();

        // Clock
        let mut rcc = init_clock(device.RCC);
        let mut delay = core.SYST.delay(&mut rcc);

        // GPIO
        let gpioa = device.GPIOA.split(&mut rcc);
        let gpiob = device.GPIOB.split(&mut rcc);

        // LCD
        let interface = LcdInterface::new(
            gpiob.pb0.into_push_pull_output().set_speed(Speed::VeryHigh),
            gpiob.pb1.into_push_pull_output().set_speed(Speed::VeryHigh),
            gpiob.pb2.into_push_pull_output().set_speed(Speed::VeryHigh),
            gpiob.pb3.into_push_pull_output().set_speed(Speed::VeryHigh),
            gpiob.pb4.into_push_pull_output().set_speed(Speed::VeryHigh),
            gpiob.pb5.into_push_pull_output().set_speed(Speed::VeryHigh),
            gpiob.pb6.into_push_pull_output().set_speed(Speed::VeryHigh),
            gpiob.pb7.into_push_pull_output().set_speed(Speed::VeryHigh),
            gpiob.pb8.into_push_pull_output().set_speed(Speed::VeryHigh),
            gpiob.pb9.into_push_pull_output().set_speed(Speed::VeryHigh),
        );
        let lcd = init_lcd(
            interface,
            gpioa.pa4.into_push_pull_output(),
            gpioa.pa5.into_push_pull_output(),
            (TOP_SCROLL_OFFSET, BOTTOM_SCROLL_OFFSET),
            &mut delay,
        )
        .unwrap();
        let display = Display::new(lcd, consumer).unwrap();
        let frame_timer = FrameTimer::new(device.TIM6, 30.hz(), &mut rcc);

        // ADC
        let dma = device.DMA.split(&mut rcc, device.DMAMUX);
        let mut ch1 = dma.ch1;
        ch1.mux().select_peripheral(DmaMuxIndex::ADC);
        let adc = Adc::new(
            device.ADC,
            device.TIM1,
            dma_buffer,
            AdcConfig::new(gpioa.pa0, ch1, 500.hz()),
            &mut rcc,
            &mut delay,
        );
        let sampler = Sampler::new(dma_buffer, producer, get_calibration(), 4095);

        // Beat counting
        let beat_timer = BeatTimer::new(device.TIM7, 10_000.ms(), &mut rcc);
        let beat_counter = BeatCounter::new(device.TIM3, gpioa.pa6, &mut rcc);

        init::LateResources {
            display,
            sampler,
            frame_timer,
            adc,
            beat_timer,
            beat_counter,
        }
    }

    #[idle(resources = [frame_timer, adc, beat_counter, beat_timer])]
    fn idle(mut cx: idle::Context) -> ! {
        cx.resources
            .beat_counter
            .lock(|counter: &mut BeatCounter| counter.start());
        cx.resources.beat_timer.lock(|timer: &mut BeatTimer| {
            timer.start();
        });
        cx.resources.frame_timer.lock(|timer: &mut FrameTimer| {
            timer.start();
        });
        cx.resources.adc.lock(|adc: &mut Adc| {
            adc.start();
        });
        loop {
            cortex_m::asm::nop();
        }
    }

    #[task(binds = DMA_CHANNEL1, priority = 2, resources = [adc, sampler])]
    fn dma(cx: dma::Context) {
        let adc: &mut Adc = cx.resources.adc;
        let sampler: &mut Sampler<'_, 64> = cx.resources.sampler;

        adc.unpend();
        sampler.sample::<IliError>().unwrap();
    }

    #[task(binds = TIM6, priority = 1, resources = [display, frame_timer])]
    fn tim6(cx: tim6::Context) {
        let frame_timer: &mut FrameTimer = cx.resources.frame_timer;
        let display: &mut Display<'_, _, _, 64> = cx.resources.display;

        frame_timer.unpend();
        display.frame().unwrap();
    }

    #[task(binds = TIM7, priority = 1, resources = [beat_counter, beat_timer, display])]
    fn tim7(cx: tim7::Context) {
        let counter: &mut BeatCounter = cx.resources.beat_counter;
        let timer: &mut BeatTimer = cx.resources.beat_timer;
        let display: &mut Display<'_, _, _, 64> = cx.resources.display;

        timer.unpend();
        display.update_bpm(counter.read() * 6).unwrap();
        counter.reset();
    }
};
