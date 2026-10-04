#![no_std]
#![no_main]

mod onewire;
mod ds18b20;
mod i2c;

use esp_backtrace as _;
esp_bootloader_esp_idf::esp_app_desc!();
use onewire::OneWire;
use 12c::I2c;
use esp_hal::{
    clock::CpuClock, 
    gpio::{Flex, InputConfig, OutputConfig, DriveMode, Pull},
    delay::Delay,
    main,
};

fn config_pin () -> Flex {
    let output_config = OutputConfig::default().with_drive_mode(DriveMode::OpenDrain).with_pull(Pull::Up);
    let input_config = InputConfig::default().with_pull(Pull::Up);

    let mut pin = Flex::new(peripherals.GPIO5);
    pin.apply_output_config(&output_config);
    pin.apply_input_config(&input_config);
    pin.set_input_enable(true);
    pin.set_output_enable(true);
    pin
}

#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::default());
    let peripherals = esp_hal::init(config);

    let mut bus = OneWire::new(&mut config_pin());
    let mut i2c = I2c::new(&mut config_pin(), &mut config_pin(), 100)

    let mut delay = Delay::new();
    let mut count = 0;

    loop {
        let temp = ds18b20::read_temperature(&mut bus, &mut delay).unwrap();
        esp_println::println!("{}: Temperature: {:.2}°C", count, temp);
        count += 1;
    }
}