#[cfg(test)]
#[embedded_test::tests(executor = esp_rtos::embassy::Executor::new())]
mod tests {
    use crate::features::simulated_esp32::{
        Application, DISPLAY_EVENTS, build_application, run_application,
        run_application_with_restartable_sessions, sessions, time, user_input,
    };
    use embassy_executor::Spawner;
    use habits_stepper::application::Configuration;
    use habits_stepper::display::DisplayEvent;
    use habits_stepper::sessions::{Session, WeekTotals};

    static CONFIGURATION: Configuration = Configuration {
        ssid: "skynet",
        pass: "test password",
        wifi_transmit_power: "18",
        metrics_url: None,
        jwt_token: None,
        chain_id: None,
    };

    fn get_local_epoch(day: i8, hour: i8) -> u32 {
        jiff::civil::date(2026, 1, day)
            .at(hour, 0, 0, 0)
            .to_zoned(habits_stepper::time::TIMEZONE)
            .expect("convert local time")
            .timestamp()
            .as_second() as u32
    }

    fn get_recent_sessions() -> [Session; 2] {
        let previous_week = get_local_epoch(11, 23);
        let current_week = get_local_epoch(12, 1);
        [
            Session {
                start_epoch: previous_week,
                end_epoch: previous_week + 1800,
                steps: 300,
            },
            Session {
                start_epoch: current_week,
                end_epoch: current_week + 1200,
                steps: 200,
            },
        ]
    }

    #[init]
    fn initialize_saved_activity() -> Application {
        build_application(
            &CONFIGURATION,
            get_local_epoch(12, 12),
            &get_recent_sessions(),
        )
    }

    fn initialize_rotation_sessions(count: u32, epoch: u32) -> Application {
        let mut application = build_application(&CONFIGURATION, epoch, &[]);
        let oldest = get_local_epoch(5, 1);
        for index in 0..count {
            let start = oldest + index * 120;
            let steps = if index < 256 { 1 } else { 2 };
            application.flash.write_session(start, start, steps);
        }
        application
    }

    fn initialize_full_storage() -> Application {
        initialize_rotation_sessions(511, get_local_epoch(12, 12))
    }

    fn initialize_before_rotation() -> Application {
        let next_session = get_local_epoch(5, 1) + 257 * 120;
        initialize_rotation_sessions(257, next_session)
    }

    async fn start(application: Application, minutes: u32, steps: u32, restartable: bool) {
        let spawner = unsafe { Spawner::for_current_executor().await };
        if restartable {
            run_application_with_restartable_sessions(spawner, application).await;
        } else {
            run_application(spawner, application).await;
        }
        wait_for_display(&DisplayEvent::Update(WeekTotals { minutes, steps })).await;
    }

    async fn wait_for_display(expected: &DisplayEvent) {
        loop {
            if DISPLAY_EVENTS
                .take_matching(|event| event == expected)
                .is_some()
            {
                return;
            }
            DISPLAY_EVENTS.wait_for_change().await;
        }
    }

    async fn wait_for_update() {
        loop {
            if DISPLAY_EVENTS
                .take_matching(|event| matches!(event, DisplayEvent::Update(_)))
                .is_some()
            {
                return;
            }
            DISPLAY_EVENTS.wait_for_change().await;
        }
    }

    async fn record_step(
        rtc: &embassy_sync::mutex::Mutex<
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            esp_hal::rtc_cntl::Rtc<'static>,
        >,
        epoch: u32,
    ) {
        time::set_time(rtc, epoch).await;
        user_input::take_step().await;
        wait_for_update().await;
    }

    async fn record_session(
        rtc: &embassy_sync::mutex::Mutex<
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            esp_hal::rtc_cntl::Rtc<'static>,
        >,
        epoch: u32,
        seconds: u32,
        steps: u32,
        history: WeekTotals,
        current: WeekTotals,
    ) {
        for index in 0..steps {
            let elapsed = index * seconds / (steps - 1);
            record_step(rtc, epoch + elapsed).await;
        }
        time::set_time(rtc, epoch + seconds + 60).await;
        check_history_and_current_week(history, current).await;
    }

    async fn check_history_and_current_week(history: WeekTotals, current: WeekTotals) {
        user_input::press_history().await;
        wait_for_display(&DisplayEvent::History(history)).await;
        user_input::release_history().await;
        wait_for_display(&DisplayEvent::Update(current)).await;
    }

    #[test]
    #[timeout(15)]
    async fn user_sees_saved_activity_after_restart(application: Application) {
        start(application, 20, 200, false).await;
        check_history_and_current_week(
            WeekTotals {
                minutes: 30,
                steps: 300,
            },
            WeekTotals {
                minutes: 20,
                steps: 200,
            },
        )
        .await;
    }

    #[test(init = initialize_full_storage)]
    #[timeout(30)]
    async fn user_sees_recent_activity_after_recording_512_sessions(application: Application) {
        let rtc = application.rtc;
        start(application, 0, 0, true).await;
        verify_rotation(rtc).await;
    }

    async fn verify_rotation(
        rtc: &embassy_sync::mutex::Mutex<
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            esp_hal::rtc_cntl::Rtc<'static>,
        >,
    ) {
        time::set_time(rtc, get_local_epoch(12, 12)).await;
        let before_rotation = WeekTotals {
            minutes: 0,
            steps: 766,
        };
        let after_rotation = WeekTotals {
            minutes: 0,
            steps: 510,
        };
        check_history_and_current_week(
            before_rotation,
            WeekTotals {
                minutes: 0,
                steps: 0,
            },
        )
        .await;
        let epoch = get_local_epoch(12, 12);
        let first_session = WeekTotals {
            minutes: 5,
            steps: 50,
        };
        record_session(rtc, epoch, 300, 50, after_rotation, first_session).await;
        sessions::restart_sessions().await;
        check_history_and_current_week(after_rotation, first_session).await;
        record_session(
            rtc,
            epoch + 480,
            60,
            10,
            after_rotation,
            WeekTotals {
                minutes: 6,
                steps: 60,
            },
        )
        .await;
    }

    #[test(init = initialize_before_rotation)]
    #[timeout(90)]
    async fn user_sees_recent_activity_after_restart_and_storage_rotation(
        application: Application,
    ) {
        let rtc = application.rtc;
        start(application, 0, 258, true).await;
        let oldest = get_local_epoch(5, 1);
        for index in 257..511 {
            let epoch = oldest + index * 120;
            record_step(rtc, epoch).await;
            record_step(rtc, epoch + 10).await;
        }
        verify_rotation(rtc).await;
    }
}
