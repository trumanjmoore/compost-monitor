#![no_std]
#![no_main]

mod onewire;
mod ds18b20;
mod i2c;
mod adastemma;

use esp_backtrace as _;
use esp_alloc as _;
esp_bootloader_esp_idf::esp_app_desc!();
use onewire::OneWire;
use i2c::I2c;
use esp_hal::{
    clock::CpuClock, 
    interrupt::software::SoftwareInterruptControl,
    ram,
    rng::Rng,
    timer::timg::TimerGroup,
    gpio::{Flex, InputConfig, OutputConfig, DriveMode, Pull},
    delay::Delay,
    rtc_cntl::sleep::{LowPower, RtcSleepConfig},
    time::{Instant},
    main,
};
use embedded_hal::delay::DelayNs;
use embassy_executor::Spawner;
use embassy_net::{
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState},
    Runner, StackResources,
};
use embassy_time::{Duration, Timer};
use esp_radio::wifi::{
    scan::ScanConfig, sta::StationConfig, Config, ControllerConfig, Interface, WifiController,
};
use reqwless::{
    client::HttpClient,
    request::{Method, RequestBuilder},
};

macro_rules! mk_static {
    ($t:ty, $val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        STATIC_CELL.uninit().write($val)
    }};
}

const SSID: &str = match option_env!("SSID") {
    Some(ssid) => ssid,
    None => "",
};
const PASSWORD: &str = match option_env!("PASSWORD") {
    Some(password) => password,
    None => "",
};

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    loop {
        if let Err(e) = controller.connect_async().await {
            esp_println::println!("Wi-Fi connect failed: {:?}", e);
        }
        controller.wait_for_event(esp_radio::wifi::WifiEvent::StaDisconnected).await;
        Timer::after(Duration::from_secs(5)).await;
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, esp_radio::wifi::Interface>) -> ! {
    runner.run().await
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::default());
    let peripherals = esp_hal::init(config);

    let output_config = OutputConfig::default().with_drive_mode(DriveMode::OpenDrain).with_pull(Pull::Up);
    let input_config = InputConfig::default().with_pull(Pull::Up);

    let mut dspin = Flex::new(peripherals.GPIO5);
    dspin.apply_output_config(&output_config);
    dspin.apply_input_config(&input_config);
    dspin.set_input_enable(true);
    dspin.set_output_enable(true);

    let mut sda = Flex::new(peripherals.GPIO7);
    sda.apply_output_config(&output_config);
    sda.apply_input_config(&input_config);
    sda.set_input_enable(true);
    sda.set_output_enable(true);

    let mut scl = Flex::new(peripherals.GPIO8);
    scl.apply_output_config(&output_config);
    scl.apply_input_config(&input_config);
    scl.set_input_enable(true);
    scl.set_output_enable(true);

    let mut bus = OneWire::new(&mut dspin);
    let mut i2c = I2c::new(&mut sda, &mut scl, 100);

    let mut delay = Delay::new();
    let mut count = 0;

    esp_println::logger::init_logger_from_env();

    esp_alloc::heap_allocator!(#[ram(reclaimed)] size: 64 * 1024);
    esp_alloc::heap_allocator!(size: 36 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    let station_config = Config::Station(StationConfig::default().with_ssid(SSID).with_password(PASSWORD.into()));

    println!("Starting Wi-Fi");
    let (mut controller, interfaces) = esp_radio::wifi::new(peripherals.WIFI, ControllerConfig::default().with_initial_config(station_config)).unwrap();
    println!("Wi-Fi configured and started");

    let wifi_interface = interfaces.station;
    let config = embassy_net::Config::dhcpv4(Default::default());

    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let (stack, runner) = embassy_net::new(wifi_interface, config, mk_static!(StackResources<3>, StackResources::<3>::new()), seed);

    println!("Scanning for access points");
    let scan_config = ScanConfig::default().with_max(10);
    let result = controller.scan_async(&scan_config).await.unwrap();
    for ap in result {
        println!("{:?}", ap);
    }

    spawner.spawn(connection(controller).unwrap());
    spawner.spawn(net_task(runner).unwrap());

    stack.wait_config_up().await;
    if let Some(config) = stack.config_v4() {
        println!("Got IP: {}", config.address);
    }


    loop {
        match ds18b20::read_temperature(&mut bus, &mut delay) {
            Some(temp) => esp_println::println!("{}: Temperature: {:.2}°C", count, temp),
            None => esp_println::println!("{}: Error: DS18B20 Not Found", count),
        }

        match adastemma::read_moisture(&mut i2c, &mut delay) {
            Some(moisture) => esp_println::println!("Moisture: {}", moisture),
            None => esp_println::println!("Error: STEMMA not found"),
        }
        count += 1;
        delay.delay_ms(5000u32);
    }
    
    delay.delay_ms(1000u32);

    //let mut lpwr = LowPower::new(peripherals.LPWR);
    //lpwr.set_wakeup_deadline(Instant::now() + Duration::from_secs(30));
    //lpwr.sleep_deep(RtcSleepConfig::deep());

    loop {}
}