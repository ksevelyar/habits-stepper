use core::sync::atomic::AtomicBool;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::rtc_cntl::Rtc;
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::time::Rate;
use habits_stepper::application::Configuration;
use habits_stepper::display::DisplayHardware;
use habits_stepper::sessions::Session;
use habits_stepper::sessions::storage::FlashRing;

pub static WIFI_CONNECTED: AtomicBool = AtomicBool::new(false);
pub static DISPLAY_EVENTS: display::DisplayEvents = display::DisplayEvents::new();

pub mod display;
pub mod sessions;
pub mod sync;
pub mod time;
pub mod user_input;
pub mod wifi;

pub struct Application {
    pub configuration: &'static habits_stepper::application::Configuration,
    pub wifi: wifi::WifiController,
    pub time: time::TimeNetwork,
    pub display: (
        habits_stepper::display::DisplayHardware,
        &'static display::DisplayEvents,
    ),
    pub rtc: &'static embassy_sync::mutex::Mutex<
        CriticalSectionRawMutex,
        esp_hal::rtc_cntl::Rtc<'static>,
    >,
    pub flash: habits_stepper::sessions::storage::FlashRing<'static>,
}

pub fn build_application(
    configuration: &'static Configuration,
    epoch: u32,
    saved_sessions: &[Session],
) -> Application {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let timer = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    let interrupts =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timer.timer0, interrupts.software_interrupt0);
    esp_alloc::heap_allocator!(size: 32768);

    DISPLAY_EVENTS.clear();
    let rtc = Rtc::new(peripherals.LPWR);
    rtc.set_current_time_us(0);
    let rtc = habits_stepper::time::RTC.init(Mutex::new(rtc));

    let mut flash = FlashRing::create(peripherals.FLASH);
    flash.erase_all();
    flash.init();
    for session in saved_sessions {
        flash.write_session(session.start_epoch, session.end_epoch, session.steps);
    }

    let display_spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(4))
            .with_mode(Mode::_0),
    )
    .expect("initialize display SPI")
    .with_sck(peripherals.GPIO5)
    .with_mosi(peripherals.GPIO6);
    let display = DisplayHardware {
        spi: display_spi,
        chip_select: Output::new(peripherals.GPIO10, Level::High, OutputConfig::default()),
        data_command: Output::new(peripherals.GPIO9, Level::Low, OutputConfig::default()),
        reset: Output::new(peripherals.GPIO7, Level::High, OutputConfig::default()),
    };
    Application {
        configuration,
        wifi: wifi::WifiController {
            connected: &WIFI_CONNECTED,
        },
        time: time::TimeNetwork {
            epoch_microseconds: u64::from(epoch) * 1_000_000,
        },
        display: (display, &DISPLAY_EVENTS),
        rtc,
        flash,
    }
}

pub async fn wait_for_display(expected: &habits_stepper::display::DisplayEvent) {
    loop {
        if DISPLAY_EVENTS.snapshot().contains(expected) {
            DISPLAY_EVENTS.clear();
            return;
        }
        embassy_time::Timer::after(embassy_time::Duration::from_millis(10)).await;
    }
}

pub async fn run_application(spawner: embassy_executor::Spawner, application: Application) {
    run_application_with_sessions(spawner, application, false).await;
}

pub async fn run_application_with_restartable_sessions(
    spawner: embassy_executor::Spawner,
    application: Application,
) {
    run_application_with_sessions(spawner, application, true).await;
}

async fn run_application_with_sessions(
    spawner: embassy_executor::Spawner,
    application: Application,
    restartable: bool,
) {
    time::seed_from_rtc(&*application.rtc.lock().await);
    spawner.spawn(time::run_sleep_task(application.rtc).expect("create sleep task"));
    spawner.spawn(display::run_display_task(application.display).expect("create display task"));
    if restartable {
        spawner.spawn(
            sessions::run_restartable_sessions_task(application.flash)
                .expect("create restartable sessions task"),
        );
    } else {
        spawner.spawn(
            habits_stepper::sessions::run_sessions_task(application.flash)
                .expect("create sessions task"),
        );
    }
    spawner.spawn(
        wifi::run_wifi_task(application.wifi, application.configuration)
            .expect("create Wi-Fi task"),
    );
    spawner.spawn(wifi::run_network_task().expect("create network task"));
    spawner.spawn(
        time::run_sync_time_task(application.time, application.rtc, application.configuration)
            .expect("create time task"),
    );
    spawner.spawn(sync::run_sync_totals_task(application.configuration).expect("create sync task"));
}
