use esp_hal::{
    delay::Delay,
    gpio::{Flex, InputConfig, OutputConfig, DriveMode, Pull},
};
use embedded_hal::delay::DelayNs;

pub struct I2c<'a> {
    sda: &'a mut Flex<'a>,
    scl: &'a mut Flex<'a>,
    delay_time: u32,
}

impl<'a> I2c<'a> {
    pub fn new(sda: &'a mut Flex<'a>, scl: &'a mut Flex<'a>, freq: u32) -> Self {
        Self {sda, scl, delay_time: 500 / freq,}
    }

    pub fn start(&mut self, delay: &mut Delay) {
        self.sda.set_high();
        self.scl.set_high();
        delay.delay_us(self.delay_time);
        self.sda.set_low();
        delay.delay_us(self.delay_time);
        self.scl.set_low();
        delay.delay_us(self.delay_time);
    }
    
    pub fn stop(&mut self, delay: &mut Delay) {
        self.sda.set_low();
        self.scl.set_high();
        delay.delay_us(self.delay_time);
        self.sda.set_high();
        delay.delay_us(self.delay_time);
    }

    fn read_bit(&mut self, delay: &mut Delay) -> bool {
        critical_section::with(|_| {
            self.sda.set_high();
            delay.delay_us(self.delay_time);
            self.scl.set_high();
            delay.delay_us(self.delay_time);
            let bit = self.sda.is_high();
            self.scl.set_low();
            delay.delay_us(self.delay_time);
            bit
        })
    }

    pub fn read_byte(&mut self, delay: &mut Delay) -> u8 {
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
            if bit {
                self.sda.set_high();
            } 
            else {
                self.sda.set_low();
            }
            delay.delay_us(self.delay_time);
            self.scl.set_high();
            delay.delay_us(self.delay_time);
            self.scl.set_low();
            delay.delay_us(self.delay_time);
        });
    }

    pub fn write_byte(&mut self,  byte: &u8, delay: &mut Delay) -> () {
        for i in 8..0 {
            let bit: bool = (byte >> i) & 1 == 1;
            self.write_bit(bit, delay);
        }
    }

    pub fn write(&mut self, address: u8, data: &[u8], delay: &mut Delay) -> Result<(), ()> {
        self.start(delay);
        if !self.write_byte(address << 1, delay) {
            return Err(());
        }
        for &bytle in data {
            if !self.write_byte(bytle, delay) {
                return Err(());
            }
        }
        self.stop(delay);
        Ok(())
    }

    pub fn read(&mut self, address: u8, buffer: &[u8], delay: &mut Delay) -> Result<(), ()> {
        self.start(delay);
        if !self.write_byte(address << 1, delay) {
            return Err(());
        }
        for i in 8..0 {
            buffer[i] = self.read_byte(!(i == 7), delay);
        }
        self.stop(delay);
        Ok(())
    }
}