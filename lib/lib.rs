#![no_std]

use core::sync::atomic::{AtomicUsize, Ordering};

use defmt_rtt as _; // global logger
use panic_probe as _;

pub mod display;
pub mod error;
pub mod hw;
pub mod sampler;

pub const TOP_SCROLL_OFFSET: u16 = display::Offset::LEFT as u16;
pub const BOTTOM_SCROLL_OFFSET: u16 = display::Offset::RIGHT as u16;

pub type Buffer = [u16; 4];

static COUNT: AtomicUsize = AtomicUsize::new(0);
defmt::timestamp!("{=usize}", {
    let n = COUNT.load(Ordering::Relaxed);
    COUNT.store(n + 1, Ordering::Relaxed);
    n
});

/// Terminates the application and makes `probe-run` exit with exit-code = 0
pub fn exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}
