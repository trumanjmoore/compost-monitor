use crate::i2c::I2c;
use esp_hal::delay::Delay;
use embedded_hal::delay::DelayNs;

const STEMMA_ADDR: u8 = 0x36;
const TOUCH_BASE: u8 = 0x0F;
const TOUCH_MOISTURE: u8 = 0x10;

pub fn read_moisture(i2c: &mut I2c<'_>, delay: &mut Delay) -> Option<u16> {
    let mut buf = [0u8; 2];
    let mut retry_counter = 0;

    delay.delay_us(1000);

    while retry_counter < 3 {
        if i2c.write(STEMMA_ADDR, &[TOUCH_BASE, TOUCH_MOISTURE], delay).is_err() {
            retry_counter += 1;
            continue
        }

        delay.delay_us(5000);

        if i2c.read(STEMMA_ADDR, &mut buf, delay).is_err() {
            retry_counter += 1;
            continue
        }

        let moisture = u16::from_be_bytes(buf);
        if moisture != u16::MAX {
            return Some(moisture);
        }
    }
    None
}
