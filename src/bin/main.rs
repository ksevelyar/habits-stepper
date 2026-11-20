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
use habits_stepper::application::CONFIGURATION;
use habits_stepper::sessions::storage;
use habits_stepper::{display, sessions, sync, time, user_input, wifi};
use panic_rtt_target as _;

use defmt::info;
use embassy_executor::Spawner;
use embassy_sync::mutex::Mutex;

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let rtc = Rtc::new(peripherals.LPWR);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 66320);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);
    info!("init: embassy initialized");

    let rtc = time::RTC.init(Mutex::new(rtc));
    let display_spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(4))
            .with_mode(Mode::_0),
    )
    .expect("initialize display SPI")
    .with_sck(peripherals.GPIO5)
    .with_mosi(peripherals.GPIO6);
    let display = display::DisplayHardware {
        spi: display_spi,
        chip_select: Output::new(peripherals.GPIO10, Level::High, OutputConfig::default()),
        data_command: Output::new(peripherals.GPIO9, Level::Low, OutputConfig::default()),
        reset: Output::new(peripherals.GPIO7, Level::High, OutputConfig::default()),
    };
    let reed = Input::new(
        peripherals.GPIO1,
        InputConfig::default().with_pull(Pull::Up),
    );
    let history = Input::new(
        peripherals.GPIO2,
        InputConfig::default().with_pull(Pull::Up),
    );
    let flash = storage::FlashRing::create(peripherals.FLASH);

    let (controller, stack, runner) = wifi::init(peripherals.WIFI, &CONFIGURATION);

    time::seed_from_rtc(&*rtc.lock().await);
    spawner.spawn(time::run_sleep_task(rtc).expect("create sleep task"));
    spawner.spawn(user_input::run_reed_task(reed).expect("create reed task"));
    spawner.spawn(user_input::run_history_task(history).expect("create history task"));
    spawner.spawn(display::run_display_task(display).expect("create display task"));
    spawner.spawn(sessions::run_sessions_task(flash).expect("create sessions task"));
    spawner.spawn(wifi::run_wifi_task(controller, &CONFIGURATION).expect("create Wi-Fi task"));
    spawner.spawn(wifi::run_network_task(runner).expect("create network task"));
    spawner.spawn(time::run_sync_time_task(stack, rtc, &CONFIGURATION).expect("create time task"));
    spawner.spawn(sync::run_sync_totals_task(stack, &CONFIGURATION).expect("create sync task"));
}
