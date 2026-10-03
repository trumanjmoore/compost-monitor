#![no_std]
#![no_main]

mod onewire;
mod ds18b20;

use esp_backtrace as _;
esp_bootloader_esp_idf::esp_app_desc!();
use onewire::OneWire;
use esp_hal::{
    clock::CpuClock, 
    gpio::{Flex, InputConfig, OutputConfig, DriveMode, Pull},
    delay::Delay,
    main,
};

#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::default());
    let peripherals = esp_hal::init(config);

    let output_config = OutputConfig::default().with_drive_mode(DriveMode::OpenDrain).with_pull(Pull::Up);
    let input_config = InputConfig::default().with_pull(Pull::Up);

    let mut pin = Flex::new(peripherals.GPIO5);
    pin.apply_output_config(&output_config);
    pin.apply_input_config(&input_config);
    pin.set_input_enable(true);
    pin.set_output_enable(true);

    let mut bus = OneWire::new(&mut pin);
    let mut delay = Delay::new();
    let mut count = 0;

    loop {
        let temp = ds18b20::read_temperature(&mut bus, &mut delay).unwrap();
        esp_println::println!("{}: Temperature: {:.2}°C", count, temp);
        count += 1;
    }
}