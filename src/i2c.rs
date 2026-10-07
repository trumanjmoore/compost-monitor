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
            delay.delay_us(2);
            self.scl.set_high();
            delay.delay_us(self.delay_time);
            delay.delay_us(1);
            let bit = self.sda.is_high();
            self.scl.set_low();
            delay.delay_us(self.delay_time);
            bit
        })
    }

    pub fn read_byte(&mut self, ack: bool, delay: &mut Delay) -> u8 {
        let mut byte : u8 = 0;
        for _ in 0..8 {
            byte  = (byte << 1) | (self.read_bit(delay) as u8);
        }
        self.write_bit(!ack, delay);
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

    pub fn write_byte(&mut self,  byte: &u8, delay: &mut Delay) -> bool {
        for i in (0..8).rev() {
            let bit: bool = (byte >> i) & 1 == 1;
            self.write_bit(bit, delay);
        }
        !self.read_bit(delay)
    }

    pub fn write(&mut self, address: u8, data: &[u8], delay: &mut Delay) -> Result<(), ()> {
        self.start(delay);
        if !self.write_byte(&(address << 1), delay) {
            self.stop(delay);
            return Err(());
        }
        for &byte in data {
            if !self.write_byte(&byte, delay) {
                self.stop(delay);
                return Err(());
            }
        }
        self.stop(delay);
        Ok(())
    }

    pub fn read(&mut self, address: u8, buffer: &mut [u8], delay: &mut Delay) -> Result<(), ()> {
        self.start(delay);
        if !self.write_byte(&(address << 1 | 1), delay) {
            self.stop(delay);
            return Err(());
        }
        let buffer_length = buffer.len();
        for i in 0..buffer_length {
            buffer[i] = self.read_byte(!(i == buffer_length - 1), delay);
        }
        self.stop(delay);
        Ok(())
    }
}