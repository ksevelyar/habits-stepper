#[cfg(test)]
#[embedded_test::tests(executor = esp_rtos::embassy::Executor::new())]
mod tests {
    use crate::features::simulated_esp32::{
        Application, DISPLAY_EVENTS, build_application, run_application, sync, time, user_input,
        wait_for_display,
    };
    use embassy_executor::Spawner;
    use embassy_time::{Duration, Timer};
    use habits_stepper::application::Configuration;
    use habits_stepper::display::DisplayEvent;
    use habits_stepper::sessions::{Session, WeekTotals};

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

    fn get_local_epoch(day: i8, hour: i8, minute: i8) -> u32 {
        jiff::civil::date(2026, 8, day)
            .at(hour, minute, 0, 0)
            .to_zoned(habits_stepper::time::TIMEZONE)
            .expect("convert local time")
            .timestamp()
            .as_second() as u32
    }

    #[init]
    fn init() -> Application {
        build_application(&CONFIGURATION, get_local_epoch(9, 12, 0), &[])
    }

    fn initialize_with_yesterday() -> Application {
        let yesterday = get_local_epoch(8, 12, 0);
        let sessions = [Session {
            start_epoch: yesterday,
            end_epoch: yesterday + 1200,
            steps: 100,
        }];
        build_application(&CONFIGURATION, get_local_epoch(9, 12, 0), &sessions)
    }

    fn initialize_without_backend() -> Application {
        build_application(
            &CONFIGURATION_WITHOUT_BACKEND,
            get_local_epoch(9, 12, 0),
            &[],
        )
    }

    fn initialize_before_midnight() -> Application {
        build_application(&CONFIGURATION, get_local_epoch(9, 23, 40), &[])
    }

    async fn start(application: Application, minutes: u32, steps: u32) {
        let spawner = unsafe { Spawner::for_current_executor().await };
        run_application(spawner, application).await;
        wait_for_display(&DisplayEvent::Update(WeekTotals { minutes, steps })).await;
    }

    async fn wait_for_update() {
        loop {
            if DISPLAY_EVENTS
                .snapshot()
                .iter()
                .any(|event| matches!(event, DisplayEvent::Update(_)))
            {
                DISPLAY_EVENTS.clear();
                return;
            }
            Timer::after(Duration::from_millis(10)).await;
        }
    }

    async fn record_steps(
        rtc: &embassy_sync::mutex::Mutex<
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            esp_hal::rtc_cntl::Rtc<'static>,
        >,
        start_epoch: u32,
        count: u32,
    ) {
        for index in 0..count {
            time::set_time(rtc, start_epoch + index * 30).await;
            user_input::take_step().await;
            wait_for_update().await;
        }
    }

    async fn complete_session(
        rtc: &embassy_sync::mutex::Mutex<
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            esp_hal::rtc_cntl::Rtc<'static>,
        >,
        last_step_epoch: u32,
    ) {
        time::set_time(rtc, last_step_epoch + 60).await;
        Timer::after(Duration::from_millis(1200)).await;
    }

    async fn wait_for_requests(count: usize) -> heapless::Vec<heapless::Vec<u8, 1024>, 8> {
        let mut received = heapless::Vec::new();
        while received.len() < count {
            for request in sync::take_requests() {
                received.push(request).expect("request capacity");
            }
            if received.len() < count {
                Timer::after(Duration::from_millis(10)).await;
            }
        }
        received
    }

    fn assert_request(request: &[u8], date: &str, minutes: &str) {
        let request = core::str::from_utf8(request).expect("request is UTF-8");
        defmt::assert!(request.contains("POST /metrics HTTP/1.1"));
        defmt::assert!(request.contains("authorization: Bearer test token"));
        defmt::assert!(request.contains("Content-Type: application/json"));
        defmt::assert!(request.contains(date));
        defmt::assert!(request.contains(minutes));
        defmt::assert!(request.contains("\"chain_id\":19"));
    }

    #[test(init = initialize_with_yesterday)]
    #[timeout(15)]
    async fn device_sends_todays_and_yesterdays_totals_after_session(application: Application) {
        let rtc = application.rtc;
        start(application, 20, 100).await;
        let epoch = get_local_epoch(9, 12, 0);
        record_steps(rtc, epoch, 21).await;
        complete_session(rtc, epoch + 600).await;
        let requests = wait_for_requests(2).await;
        assert_request(&requests[0], "\"date\":\"2026-08-09\"", "\"value\":\"10\"");
        assert_request(&requests[1], "\"date\":\"2026-08-08\"", "\"value\":\"20\"");
    }

    #[test(init = initialize_without_backend)]
    #[timeout(15)]
    async fn device_sends_no_totals_without_metrics_url(application: Application) {
        let rtc = application.rtc;
        start(application, 0, 0).await;
        let epoch = get_local_epoch(9, 12, 0);
        record_steps(rtc, epoch, 3).await;
        complete_session(rtc, epoch + 60).await;
        defmt::assert!(sync::take_requests().is_empty());
    }

    #[test]
    #[timeout(15)]
    async fn device_sends_sum_of_sessions_on_same_day(application: Application) {
        let rtc = application.rtc;
        start(application, 0, 0).await;
        let epoch = get_local_epoch(9, 12, 0);
        record_steps(rtc, epoch, 3).await;
        complete_session(rtc, epoch + 60).await;
        let _ = wait_for_requests(2).await;
        record_steps(rtc, epoch + 180, 3).await;
        complete_session(rtc, epoch + 240).await;
        let requests = wait_for_requests(2).await;
        assert_request(&requests[0], "\"date\":\"2026-08-09\"", "\"value\":\"2\"");
        assert_request(&requests[1], "\"date\":\"2026-08-08\"", "\"value\":\"0\"");
    }

    #[test(init = initialize_before_midnight)]
    #[timeout(20)]
    async fn device_sends_session_across_midnight_on_both_dates(application: Application) {
        let rtc = application.rtc;
        start(application, 0, 0).await;
        let epoch = get_local_epoch(9, 23, 40);
        record_steps(rtc, epoch, 81).await;
        complete_session(rtc, epoch + 2400).await;
        let requests = wait_for_requests(2).await;
        assert_request(&requests[0], "\"date\":\"2026-08-10\"", "\"value\":\"20\"");
        assert_request(&requests[1], "\"date\":\"2026-08-09\"", "\"value\":\"20\"");
    }
}
