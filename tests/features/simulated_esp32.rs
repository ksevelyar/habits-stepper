use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

pub mod display;
pub mod sync;
pub mod time;
pub mod wifi;

pub use habits_stepper::{sessions, user_input};

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
    pub reed: esp_hal::gpio::Input<'static>,
    pub history: esp_hal::gpio::Input<'static>,
    pub flash: habits_stepper::sessions::storage::FlashRing<'static>,
}

pub async fn run_application(spawner: embassy_executor::Spawner, application: Application) {
    time::seed_from_rtc(&*application.rtc.lock().await);
    spawner.spawn(time::run_sleep_task(application.rtc).expect("create sleep task"));
    spawner.spawn(user_input::run_reed_task(application.reed).expect("create reed task"));
    spawner.spawn(user_input::run_history_task(application.history).expect("create history task"));
    spawner.spawn(display::run_display_task(application.display).expect("create display task"));
    spawner.spawn(sessions::run_sessions_task(application.flash).expect("create sessions task"));
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
