#![no_std]
#![no_main]

use esp_alloc as _;
use esp_hal::{
    clock::CpuClock,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    rtc_cntl::Rtc,
    spi::Mode,
    spi::master::{Config as SpiConfig, Spi},
    time::Rate,
    timer::timg::TimerGroup,
};
use habits_stepper::{display, sessions, storage, time, user_input, wifi};
use habits_stepper::sessions::FlashMutex;
use panic_rtt_target as _;

use defmt::info;
use embassy_executor::Spawner;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let rtc = Rtc::new(peripherals.LPWR);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 66320);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);
    info!("Embassy initialized!");

    let (controller, stack, runner) = wifi::init(peripherals.WIFI).await;

    spawner.spawn(wifi::connection(controller).unwrap());
    spawner.spawn(wifi::net_task(runner).unwrap());
    spawner.spawn(time::task(rtc, stack).unwrap());
    let display_spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(4))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO6)
    .with_mosi(peripherals.GPIO7);

    let display_cs = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());
    let display_dc = Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default());
    let display_rst = Output::new(peripherals.GPIO3, Level::High, OutputConfig::default());

    let reed = Input::new(
        peripherals.GPIO1,
        InputConfig::default().with_pull(Pull::Up),
    );
    let history = Input::new(
        peripherals.GPIO2,
        InputConfig::default().with_pull(Pull::Up),
    );

    spawner.spawn(user_input::task(reed, history).unwrap());
    spawner.spawn(display::display_task(display_spi, display_cs, display_dc, display_rst).unwrap());
    static FLASH_RING: StaticCell<FlashMutex> = StaticCell::new();
    let flash_mutex = FLASH_RING.init(Mutex::new(storage::FlashRing::new(peripherals.FLASH)));

    spawner.spawn(sessions::session_task(flash_mutex).unwrap());
    spawner.spawn(sessions::sync_task(flash_mutex).unwrap());

    loop {
        Timer::after(Duration::from_secs(3600)).await;
    }
}
