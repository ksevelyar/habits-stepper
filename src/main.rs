mod sessions;

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{AnyIOPin, AnyOutputPin, Output, PinDriver, Pull};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::spi::{config::DriverConfig, SpiDeviceDriver, SpiDriver};
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sntp;
use esp_idf_svc::sys::EspError;
use esp_idf_svc::wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi};
use log::info;
use sessions::{new_storage, Sessions};
use sh1122::{Framebuffer, Sh1122Device, Sh1122Interface};
use std::time::SystemTime;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASS");

const DISPLAY_WIDTH: usize = 256;
const DISPLAY_HEIGHT: usize = 64;
const BRIGHTNESS: u8 = 0x60;
const DIGIT_SEGMENTS: [u8; 10] = [
    0b0111111, 0b0000110, 0b1011011, 0b1001111, 0b1100110, 0b1101101, 0b1111101, 0b0000111,
    0b1111111, 0b1101111,
];

struct HardSpi {
    spi: SpiDeviceDriver<'static, SpiDriver<'static>>,
    cs: PinDriver<'static, Output>,
    dc: PinDriver<'static, Output>,
}

impl HardSpi {
    fn new(
        spi: SpiDeviceDriver<'static, SpiDriver<'static>>,
        pin_cs: impl esp_idf_svc::hal::gpio::OutputPin + 'static,
        pin_dc: impl esp_idf_svc::hal::gpio::OutputPin + 'static,
    ) -> Result<Self, EspError> {
        Ok(Self {
            spi,
            cs: PinDriver::output(pin_cs)?,
            dc: PinDriver::output(pin_dc)?,
        })
    }
}

impl Sh1122Interface for HardSpi {
    fn write_cmd(&mut self, command: u8, data: &[u8]) -> anyhow::Result<()> {
        self.cs.set_low()?;
        self.dc.set_low()?;
        self.spi.write(&[command])?;
        if !data.is_empty() {
            self.spi.write(data)?;
        }
        self.cs.set_high()?;
        Ok(())
    }

    fn write_data(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.cs.set_low()?;
        self.dc.set_high()?;
        self.spi.write(data)?;
        self.cs.set_high()?;
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    let utc_offset: i32 = env!("UTC_OFFSET").parse().unwrap_or(180);
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
    let spi_driver = SpiDriver::new(
        peripherals.spi2,
        peripherals.pins.gpio6,
        peripherals.pins.gpio7,
        AnyIOPin::none(),
        &DriverConfig::default(),
    )?;

    let spi = SpiDeviceDriver::new(spi_driver, None::<AnyOutputPin>, &Default::default())?;

    let spi_interface = HardSpi::new(spi, peripherals.pins.gpio10, peripherals.pins.gpio4)?;
    let mut display = Sh1122Device::with_interface(spi_interface, DISPLAY_WIDTH, DISPLAY_HEIGHT);

    display.init_display().ok();
    let reed = PinDriver::input(peripherals.pins.gpio2, Pull::Up)?;

    let mut last_trigger_ms: u64 = 0;
    let mut last_reed_high: bool = true;

    loop {
        let now_ms = get_now_ms();
        let current_low = reed.is_low();

        if current_low && last_reed_high {
            if now_ms.saturating_sub(last_trigger_ms) > 50 {
                last_trigger_ms = now_ms;
                sm.trigger(now_ms);
                info!("today minutes: {}", sm.today_minutes(now_ms));
            }
        }

        sm.tick(now_ms);

        display.fill(0);
        let mins = sm.today_minutes(now_ms);
        let tens = (mins / 10) as usize;
        let ones = (mins % 10) as usize;
        draw_digit(&mut display, DIGIT_SEGMENTS[tens.min(9)], 0, BRIGHTNESS);
        draw_colon(&mut display, 26, BRIGHTNESS);
        draw_digit(&mut display, DIGIT_SEGMENTS[ones.min(9)], 40, BRIGHTNESS);

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

fn draw_digit<D: Sh1122Interface>(display: &mut Sh1122Device<D>, bits: u8, x: usize, color: u8) {
    if bits & 1 != 0 {
        draw_rect(display, x + 10, 0, 14, 4, color);
    }
    if bits & 2 != 0 {
        draw_rect(display, x + 24, 4, 4, 26, color);
    }
    if bits & 4 != 0 {
        draw_rect(display, x + 24, 34, 4, 26, color);
    }
    if bits & 8 != 0 {
        draw_rect(display, x + 10, 60, 14, 4, color);
    }
    if bits & 16 != 0 {
        draw_rect(display, x + 8, 34, 4, 26, color);
    }
    if bits & 32 != 0 {
        draw_rect(display, x + 8, 4, 4, 26, color);
    }
    if bits & 64 != 0 {
        draw_rect(display, x + 10, 30, 14, 4, color);
    }
}

fn draw_colon<D: Sh1122Interface>(display: &mut Sh1122Device<D>, x: usize, color: u8) {
    draw_rect(display, x + 10, 26, 4, 4, color);
    draw_rect(display, x + 10, 34, 4, 4, color);
}

fn draw_rect<D: Sh1122Interface>(
    display: &mut Sh1122Device<D>,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: u8,
) {
    for xi in x..x + width {
        for yi in y..y + height {
            display.set_pixel(xi, yi, color);
        }
    }
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
