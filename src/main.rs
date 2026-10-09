#![no_std]
#![no_main]

mod onewire;
mod ds18b20;
mod i2c;
mod adastemma;

use esp_backtrace as _;
use esp_alloc as _;
esp_bootloader_esp_idf::esp_app_desc!();

use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Flex, InputConfig, OutputConfig, DriveMode, Pull},
    interrupt::software::SoftwareInterruptControl,
    rng::Rng,
    timer::timg::TimerGroup,
    rtc_cntl::{Rtc, sleep::TimerWakeupSource},
};
use embassy_executor::Spawner;
use embassy_net::{
    Runner,
    StackResources,
    tcp::TcpSocket,
    dns::DnsQueryType,
};
use myrtio_mqtt::{
    MqttClient,
    MqttOptions,
    QoS,
    TcpTransport,
};
use embassy_time::{Duration, Timer};
use esp_radio::wifi::{
    Config, 
    ControllerConfig, 
    WifiController, 
    sta::StationConfig,
};
use core::{fmt::Write, time::Duration as CoreDuration};
use heapless::String;
use esp_println::println;
use onewire::OneWire;
use i2c::I2c;

static mut NETWORK_RESOURCES: StackResources<3> = StackResources::new();

static mut SOCKET_RX_BUFFER: [u8; 1024] = [0; 1024];
static mut SOCKET_TX_BUFFER: [u8; 1024] = [0; 1024];

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
    println!("Connecting to Wi-Fi...");
    loop {
        match controller.connect_async().await {
            Ok(_) => println!("Wi-Fi connected!"),
            Err(e) => {
                println!("Wi-Fi connect failed: {:?}", e);
                Timer::after(Duration::from_secs(5)).await;
                continue;
            }
        }
        let _ = controller.wait_for_disconnect_async().await;
        println!("Wi-Fi disconnected. Reconnecting...");
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, esp_radio::wifi::Interface<'static>>) -> ! {
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

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 64 * 1024);
    esp_alloc::heap_allocator!(size: 36 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    let station_config = Config::Station(StationConfig::default().with_ssid(SSID).with_password(PASSWORD.into()));

    println!("Starting Wi-Fi");
    let (controller, interfaces) = esp_radio::wifi::new(peripherals.WIFI, ControllerConfig::default().with_initial_config(station_config)).unwrap();
    println!("Wi-Fi configured and started");

    let wifi_interface = interfaces.station;
    let net_config = embassy_net::Config::dhcpv4(Default::default());

    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let (stack, runner) = embassy_net::new(wifi_interface, net_config, unsafe { &mut NETWORK_RESOURCES }, seed);

    spawner.spawn(connection(controller).unwrap());
    spawner.spawn(net_task(runner).unwrap());

    stack.wait_config_up().await;
    if let Some(config) = stack.config_v4() {
        println!("Got IP: {}", config.address);
    }

    let broker_hostname = "test.mosquitto.org";
    let broker_ip = match stack.dns_query(broker_hostname, DnsQueryType::A).await {
        Ok(addrs) => addrs[0],
        Err(e) => {
            println!("DNS resolution failed: {:?}", e);
            loop {
                Timer::after(Duration::from_secs(60)).await;
            }
        }
    };
    println!("Resolved {} to {}", broker_hostname, broker_ip);

    let mut socket = TcpSocket::new(
        stack,
        unsafe { &mut SOCKET_RX_BUFFER },
        unsafe { &mut SOCKET_TX_BUFFER },
    );
    socket.set_timeout(Some(Duration::from_secs(10)));

    if let Err(e) = socket.connect((broker_ip, 1883)).await {
        println!("TCP connect failed: {:?}", e);
        loop {
            Timer::after(Duration::from_secs(60)).await;
        }
    }
    println!("TCP connected to broker");

    let transport = TcpTransport::new(socket, Duration::from_secs(5));
    let options = MqttOptions::new("compost-monitor-1");
    let mut client = MqttClient::<_, 8, 1024>::new(transport, options);

    if let Err(e) = client.connect().await {
        println!("MQTT connect failed: {:?}", e);
        loop {
            Timer::after(Duration::from_secs(60)).await;
        }
    }
    println!("MQTT connected");

    let _ = embassy_time::with_timeout(Duration::from_secs(2), client.poll()).await;

    let temp = ds18b20::read_temperature(&mut bus, &mut delay);
    let moisture = adastemma::read_moisture(&mut i2c, &mut delay);

    let mut json: String<128> = String::new();
    match (temp, moisture) {
        (Some(t), Some(m)) => {
            let _ = write!(json, "{{\"temp\":{:.2},\"moisture\":{}}}", t, m);
        }
        _ => {
            let _ = write!(json, "{{\"error\":\"sensor read failed\"}}");
        }
    }

    if let Err(e) = client.publish("compost/trumanmoore/a7f3c91b", json.as_bytes(), QoS::AtMostOnce).await {
        println!("Publish failed: {:?}", e);
    }
    println!("Published!");

    Timer::after(Duration::from_millis(500)).await;

    drop(client);

    Timer::after(Duration::from_millis(500)).await;

    // 15 min sleep
    let timer = TimerWakeupSource::new(CoreDuration::from_secs(10));

    Timer::after(Duration::from_millis(100)).await;

    let mut rtc = Rtc::new(peripherals.LPWR);
    rtc.sleep_deep(&[&timer]);

    loop {}
}