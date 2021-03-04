use heapless::spsc::{Producer, SingleCore};
use heapless::ArrayLength;

use crate::error::Error;
use crate::error::Result;
use crate::Buffer;

pub struct Sampler<'a, LEN>
where
    LEN: ArrayLength<u16>,
{
    producer: Producer<'a, u16, LEN, u8, SingleCore>,
    buffer: &'static Buffer,
    first_half: bool,
    calibration: u32,
    full_scale: u16,
}

impl<'a, LEN> Sampler<'a, LEN>
where
    LEN: ArrayLength<u16>,
{
    pub fn new(
        buffer: &'static Buffer,
        producer: Producer<'a, u16, LEN, u8, SingleCore>,
        vref_calibration: u16,
        full_scale: u16,
    ) -> Self {
        // 3V * 1000 to prevent floating math
        let calibration = vref_calibration as u32 * 3000;
        Sampler {
            producer,
            buffer,
            calibration,
            full_scale,
            first_half: true,
        }
    }

    pub fn sample<LCDER>(&mut self) -> Result<(), LCDER> {
        let (vref, input) = self.get_raw_data();
        self.first_half ^= true;
        let sample = self.convert(vref, input);
        self.producer.enqueue(sample).map_err(|_| Error::Queue)
    }

    fn get_raw_data(&self) -> (u16, u16) {
        if self.first_half {
            (self.buffer[1], self.buffer[0])
        } else {
            (self.buffer[3], self.buffer[2])
        }
    }

    fn convert(&self, measured_vref: u16, measured_input: u16) -> u16 {
        let v_ref = self.calibration / measured_vref as u32;
        let sample = (v_ref * measured_input as u32) / self.full_scale as u32;
        sample as u16
    }
}
