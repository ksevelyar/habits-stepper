#[cfg(test)]
#[embedded_test::tests(executor = esp_rtos::embassy::Executor::new())]
mod tests {
    use crate::features::simulated_esp32::display::DisplayEvents;
    use crate::features::simulated_esp32::time::TimeNetwork;
    use crate::features::simulated_esp32::wifi::WifiController;
    use crate::features::simulated_esp32::{Application, run_application};
    use core::sync::atomic::{AtomicBool, Ordering};
    use embassy_executor::Spawner;
    use embassy_sync::mutex::Mutex;
    use embassy_time::{Duration, Timer};
    use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
    use esp_hal::rtc_cntl::Rtc;
    use esp_hal::spi::Mode;
    use esp_hal::spi::master::{Config as SpiConfig, Spi};
    use esp_hal::time::Rate;
    use habits_stepper::application::Configuration;
    use habits_stepper::display::{DisplayEvent, DisplayHardware, StartupStage};
    use habits_stepper::sessions::WeekTotals;
    use habits_stepper::sessions::storage::FlashRing;
    use habits_stepper::time;

    static WIFI_CONNECTED: AtomicBool = AtomicBool::new(false);
    static DISPLAY_EVENTS: DisplayEvents = DisplayEvents::new();
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
        let peripherals = esp_hal::init(esp_hal::Config::default());
        let timer = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
        let interrupts =
            esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
        esp_rtos::start(timer.timer0, interrupts.software_interrupt0);
        esp_alloc::heap_allocator!(size: 32768);

        DISPLAY_EVENTS.clear();
        let rtc = Rtc::new(peripherals.LPWR);
        rtc.set_current_time_us(0);
        let rtc = time::RTC.init(Mutex::new(rtc));

        let mut flash = FlashRing::create(peripherals.FLASH);
        flash.erase_all();
        flash.init();
        let week_start = time::calculate_week_start(CURRENT_EPOCH);
        flash.write_session(week_start + 60, week_start + 660, 120);
        flash.write_session(week_start - 660, week_start - 60, 500);

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
        let reed = Input::new(
            peripherals.GPIO1,
            InputConfig::default().with_pull(Pull::Up),
        );
        let history = Input::new(
            peripherals.GPIO2,
            InputConfig::default().with_pull(Pull::Up),
        );
        Application {
            configuration,
            wifi: WifiController {
                connected: &WIFI_CONNECTED,
            },
            time: TimeNetwork {
                epoch_microseconds: u64::from(CURRENT_EPOCH) * 1_000_000,
            },
            display: (display, &DISPLAY_EVENTS),
            rtc,
            reed,
            history,
            flash,
        }
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
