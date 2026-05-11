mod display;
mod sessions;

use display::{BRIGHTNESS, create_display, init_spi, render_time};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{AnyIOPin, PinDriver, Pull};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sntp;
use esp_idf_svc::sys::EspError;
use esp_idf_svc::wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi};
use log::info;
use sessions::{Sessions, new_storage};
use sh1122::Framebuffer;
use std::time::SystemTime;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASS");

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    let _utc_offset: i32 = env!("UTC_OFFSET").parse().unwrap_or(180);
    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs_partition = EspDefaultNvsPartition::take()?;
    let _wifi = wifi_create(SSID, PASSWORD, peripherals.modem, sysloop)?;
    let _sntp = sntp::EspSntp::new_default()?;
    info!("SNTP initialized");

    let storage = new_storage(nvs_partition)?;
    let mut sm = Sessions::new(storage);

    let mut pin_reset = PinDriver::output(peripherals.pins.gpio3)?;
    pin_reset.set_high()?;

    let spi = init_spi(
        peripherals.spi2,
        peripherals.pins.gpio6,
        peripherals.pins.gpio7,
        AnyIOPin::none(),
    )?;
    let mut display = create_display(spi, peripherals.pins.gpio10, peripherals.pins.gpio4)?;

    let reed = PinDriver::input(peripherals.pins.gpio2, Pull::Up)?;

    let mut last_trigger_ms: u64 = 0;
    let mut last_reed_high: bool = true;

    loop {
        let now_ms = get_now_ms();
        let current_low = reed.is_low();

        if current_low && last_reed_high && now_ms.saturating_sub(last_trigger_ms) > 50 {
            last_trigger_ms = now_ms;
            sm.trigger(now_ms);
            info!("today minutes: {}", sm.today_minutes(now_ms));
        }

        sm.tick(now_ms);

        display.fill(0);
        render_time(&mut display, sm.today_minutes(now_ms), BRIGHTNESS);
        display.flush().ok();

        last_reed_high = !current_low;
        FreeRtos::delay_ms(50);
    }
}

fn get_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn wifi_create(
    ssid: &str,
    pass: &str,
    modem: esp_idf_svc::hal::modem::Modem<'static>,
    sysloop: EspSystemEventLoop,
) -> Result<EspWifi<'static>, EspError> {
    let mut esp_wifi = EspWifi::new(modem, sysloop.clone(), None)?;
    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop.clone())?;
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid.try_into().unwrap(),
        password: pass.try_into().unwrap(),
        ..Default::default()
    }))?;
    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!("Wifi DHCP info: {:?}", ip_info);
    Ok(esp_wifi)
}
