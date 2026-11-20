#[cfg(test)]
#[embedded_test::tests(executor = esp_rtos::embassy::Executor::new())]
mod tests {
    use crate::features::simulated_esp32::{
        Application, DISPLAY_EVENTS, WIFI_CONNECTED, build_application, run_application,
    };
    use core::sync::atomic::Ordering;
    use embassy_executor::Spawner;
    use embassy_sync::mutex::Mutex;
    use embassy_time::{Duration, Timer};
    use esp_hal::rtc_cntl::Rtc;
    use habits_stepper::application::Configuration;
    use habits_stepper::display::{DisplayEvent, StartupStage};
    use habits_stepper::sessions::{Session, WeekTotals};
    use habits_stepper::time;

    static CONFIGURATION: Configuration = Configuration {
        ssid: "skynet",
        pass: "test password",
        wifi_transmit_power: "18",
        metrics_url: Some("http://example.com/metrics"),
        jwt_token: Some("test token"),
        chain_id: Some("19"),
    };
    static CONFIGURATION_WITHOUT_BACKEND: Configuration = Configuration {
        metrics_url: None,
        ..CONFIGURATION
    };
    const CURRENT_EPOCH: u32 = 1_800_000_000;

    #[init]
    fn init() -> Application {
        build_test_application(&CONFIGURATION)
    }

    fn initialize_without_backend() -> Application {
        build_test_application(&CONFIGURATION_WITHOUT_BACKEND)
    }

    fn build_test_application(configuration: &'static Configuration) -> Application {
        let week_start = time::calculate_week_start(CURRENT_EPOCH);
        let sessions = [
            Session {
                start_epoch: week_start - 660,
                end_epoch: week_start - 60,
                steps: 500,
            },
            Session {
                start_epoch: week_start + 60,
                end_epoch: week_start + 660,
                steps: 120,
            },
        ];
        build_application(configuration, CURRENT_EPOCH, &sessions)
    }

    #[test]
    #[timeout(10)]
    async fn device_synchronizes_time_on_boot_with_backend(application: Application) {
        let rtc = application.rtc;
        let spawner = unsafe { Spawner::for_current_executor().await };
        run_application(spawner, application).await;
        verify_startup(rtc).await;
        defmt::assert!(WIFI_CONNECTED.load(Ordering::Acquire));
    }

    #[test(init = initialize_without_backend)]
    #[timeout(10)]
    async fn device_synchronizes_time_on_boot_without_backend(application: Application) {
        let rtc = application.rtc;
        let spawner = unsafe { Spawner::for_current_executor().await };
        run_application(spawner, application).await;
        verify_startup(rtc).await;
        Timer::after(Duration::from_millis(100)).await;
        defmt::assert!(!WIFI_CONNECTED.load(Ordering::Acquire));
    }

    async fn verify_startup(
        rtc: &'static Mutex<
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            Rtc<'static>,
        >,
    ) {
        let expected = [
            DisplayEvent::Startup(StartupStage::Wifi),
            DisplayEvent::Startup(StartupStage::Dhcp),
            DisplayEvent::Startup(StartupStage::Dns),
            DisplayEvent::Startup(StartupStage::Ntp),
            DisplayEvent::Update(WeekTotals {
                minutes: 10,
                steps: 120,
            }),
        ];
        loop {
            let events = DISPLAY_EVENTS.snapshot();
            if events.len() >= expected.len() {
                defmt::assert!(events.as_slice() == expected);
                break;
            }
            Timer::after(Duration::from_millis(10)).await;
        }
        let rtc_seconds = rtc.lock().await.current_time_us() / 1_000_000;
        defmt::assert!(rtc_seconds >= u64::from(CURRENT_EPOCH));
        defmt::assert!(rtc_seconds <= u64::from(CURRENT_EPOCH) + 5);
    }
}
