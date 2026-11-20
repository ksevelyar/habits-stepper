#[cfg(test)]
#[embedded_test::tests(executor = esp_rtos::embassy::Executor::new())]
mod tests {
    use crate::features::simulated_esp32::{
        Application, build_application, run_application, time, user_input, wait_for_display,
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

    fn get_monday_noon() -> u32 {
        jiff::civil::date(2026, 1, 12)
            .at(12, 0, 0, 0)
            .to_zoned(habits_stepper::time::TIMEZONE)
            .expect("convert local time")
            .timestamp()
            .as_second() as u32
    }

    #[init]
    fn init() -> Application {
        build_application(&CONFIGURATION, get_monday_noon(), &[])
    }

    fn initialize_history() -> Application {
        let now = get_monday_noon();
        let week_start = habits_stepper::time::calculate_week_start(now);
        let sessions = [
            Session {
                start_epoch: week_start - 1860,
                end_epoch: week_start - 60,
                steps: 300,
            },
            Session {
                start_epoch: week_start + 60,
                end_epoch: week_start + 660,
                steps: 100,
            },
        ];
        build_application(&CONFIGURATION, now, &sessions)
    }

    fn initialize_monday() -> Application {
        let now = get_monday_noon();
        let week_start = habits_stepper::time::calculate_week_start(now);
        let sessions = [Session {
            start_epoch: week_start - 7200,
            end_epoch: week_start - 3600,
            steps: 60,
        }];
        build_application(&CONFIGURATION, now, &sessions)
    }

    async fn start(application: Application, minutes: u32, steps: u32) {
        let spawner = unsafe { Spawner::for_current_executor().await };
        run_application(spawner, application).await;
        wait_for_display(&DisplayEvent::Update(WeekTotals { minutes, steps })).await;
    }

    async fn expect_update(minutes: u32, steps: u32) {
        wait_for_display(&DisplayEvent::Update(WeekTotals { minutes, steps })).await;
    }

    #[test]
    #[timeout(10)]
    async fn device_counts_steps_from_both_pedals(application: Application) {
        start(application, 0, 0).await;
        user_input::take_step().await;
        expect_update(0, 1).await;
        user_input::take_step().await;
        expect_update(0, 2).await;
    }

    #[test]
    #[timeout(10)]
    async fn user_records_session(application: Application) {
        let rtc = application.rtc;
        start(application, 0, 0).await;
        let start_epoch = get_monday_noon();
        for step in 1..=10 {
            let elapsed = (step - 1).min(8) * 15;
            time::set_time(rtc, start_epoch + elapsed).await;
            user_input::take_step().await;
            expect_update(elapsed / 60, step).await;
        }
        time::set_time(rtc, start_epoch + 180).await;
        embassy_time::Timer::after(embassy_time::Duration::from_millis(1100)).await;
        user_input::take_step().await;
        expect_update(2, 11).await;
    }

    #[test(init = initialize_history)]
    #[timeout(10)]
    async fn user_views_session_history(application: Application) {
        start(application, 10, 100).await;
        user_input::press_history().await;
        wait_for_display(&DisplayEvent::History(WeekTotals {
            minutes: 30,
            steps: 300,
        }))
        .await;
        user_input::release_history().await;
        expect_update(10, 100).await;
    }

    #[test(init = initialize_monday)]
    #[timeout(10)]
    async fn user_sees_activity_move_to_history_on_monday(application: Application) {
        start(application, 0, 0).await;
        user_input::press_history().await;
        wait_for_display(&DisplayEvent::History(WeekTotals {
            minutes: 60,
            steps: 60,
        }))
        .await;
    }
}
