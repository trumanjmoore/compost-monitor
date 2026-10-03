use crate::onewire::OneWire;
use esp_hal::delay::Delay;
use embedded_hal::delay::DelayNs;

const SKIP_ROM: u8 = 0xCC;
const CONVERT_T: u8 = 0x44;
const READ_SCRATCHPAD: u8 = 0xBE;

pub fn read_temperature(bus: &mut OneWire<'_>, delay: &mut Delay) -> Option<f32> {
    if !bus.reset(delay) { 
        return None; 
    }

    bus.write_byte(&SKIP_ROM, delay);
    bus.write_byte(&CONVERT_T, delay);
    delay.delay_ms(750u32);

    if !bus.reset(delay) { 
        return None; 
    }

    bus.write_byte(&SKIP_ROM, delay);
    bus.write_byte(&READ_SCRATCHPAD, delay);

    let lsb = bus.read_byte(delay) as u16;
    let msb = bus.read_byte(delay) as u16;
    let raw_temp = (msb << 8) | lsb;

    Some((raw_temp as i16) as f32 / 16.0)
}