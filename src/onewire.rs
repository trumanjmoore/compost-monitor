use esp_hal::{
    delay::Delay,
    gpio::Flex,
};
use embedded_hal::delay::DelayNs;

pub struct OneWire<'a> {
    pin: &'a mut Flex<'a>,
}

impl<'a> OneWire<'a> {
    pub fn new(pin: &'a mut Flex<'a>) -> Self {
        Self { pin }
    }

    pub fn reset(&mut self, delay: &mut Delay) -> bool {
        self.pin.set_low();
        delay.delay_us(480);
        self.pin.set_high();
        delay.delay_us(70);
        let alive = self.pin.is_low();
        delay.delay_us(410);
        alive
    }

    fn read_bit(&mut self, delay: &mut Delay) -> bool {
        critical_section::with(|_| {
            self.pin.set_low();
            delay.delay_us(6);
            self.pin.set_high();
            delay.delay_us(9);
            let bit = self.pin.is_high();
            delay.delay_us(55);
            return bit;
        }) 
    }

    pub fn read_byte(&mut self, delay: &mut Delay) -> u8{
        let mut byte : u8 = 0;
        for i in 0..8 {
            if self.read_bit(delay) {
                byte |= 1 << i;
            }
        }
        byte
    }

    fn write_bit(&mut self, bit: bool, delay: &mut Delay) -> () {
        critical_section::with(|_| {
            self.pin.set_low();
            if bit {
                delay.delay_us(10);
                self.pin.set_high();
                delay.delay_us(60);
            } 
            else {
                delay.delay_us(60);
                self.pin.set_high();
                delay.delay_us(10);
            }
        });
    }

    pub fn write_byte(&mut self,  byte: &u8, delay: &mut Delay) -> () {
        for i in 0..8 {
            let bit: bool = (byte >> i) & 1 == 1;
            self.write_bit(bit, delay);
        }
    }
}