#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(test)]
#[embedded_test::tests(executor = esp_rtos::embassy::Executor::new())]
mod tests {
    use defmt::assert_eq;
    use habits_stepper::sessions::{Session, Sessions, make_day_minutes, make_week_totals};
    use heapless::Deque;

    fn session_from_raw(current: Option<Session>, ring_sessions: &[Session]) -> Sessions {
        let mut history = Deque::new();
        for session in ring_sessions {
            history.push_back(*session).unwrap();
        }
        Sessions { current, history }
    }

    #[init]
    fn init() {
        let peripherals = esp_hal::init(esp_hal::Config::default());
        let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
        let sw_interrupt =
            esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
        esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);
        rtt_target::rtt_init_defmt!();
    }

    #[test]
    async fn current_week_starts_at_monday_midnight() {
        let monday_morning = jiff::civil::date(2026, 1, 12)
            .at(10, 0, 0, 0)
            .to_zoned(habits_stepper::time::TIMEZONE)
            .unwrap();
        let current_time = monday_morning.timestamp().as_second() as u32;

        let sunday_noon = jiff::civil::date(2026, 1, 11)
            .at(12, 0, 0, 0)
            .to_zoned(habits_stepper::time::TIMEZONE)
            .unwrap();
        let previous_week_session_epoch = sunday_noon.timestamp().as_second() as u32;

        let previous_week_session = Session {
            start_epoch: previous_week_session_epoch,
            end_epoch: previous_week_session_epoch + 3600,
            steps: 60,
        };
        let current_session = Session {
            start_epoch: current_time,
            end_epoch: current_time + 1800,
            steps: 10,
        };

        let sessions = session_from_raw(Some(current_session), &[previous_week_session]);

        let totals = make_week_totals(&sessions, current_time + 600);
        assert_eq!(totals.minutes, 30);
        assert_eq!(totals.steps, 10);
    }

    #[test]
    async fn prev_week_session_appears_in_history() {
        let monday_afternoon = jiff::civil::date(2026, 1, 12)
            .at(13, 20, 0, 0)
            .to_zoned(habits_stepper::time::TIMEZONE)
            .unwrap();
        let current_time = monday_afternoon.timestamp().as_second() as u32;

        let wednesday_noon = jiff::civil::date(2026, 1, 7)
            .at(12, 0, 0, 0)
            .to_zoned(habits_stepper::time::TIMEZONE)
            .unwrap();
        let previous_week_session_epoch = wednesday_noon.timestamp().as_second() as u32;

        let previous_week_session = Session {
            start_epoch: previous_week_session_epoch,
            end_epoch: previous_week_session_epoch + 3600,
            steps: 60,
        };

        let sessions = session_from_raw(None, &[previous_week_session]);

        let totals = make_week_totals(&sessions, current_time - 604800);
        assert_eq!(totals.minutes, 60);
        assert_eq!(totals.steps, 60);
    }

    #[test]
    async fn day_totals_sum_all_sessions_on_same_day() {
        let midnight = jiff::civil::date(2026, 8, 9)
            .at(0, 0, 0, 0)
            .to_zoned(habits_stepper::time::TIMEZONE)
            .unwrap();
        let day_start = midnight.timestamp().as_second() as u32;

        let morning = Session {
            start_epoch: day_start + 3600,
            end_epoch: day_start + 3660,
            steps: 10,
        };
        let evening = Session {
            start_epoch: day_start + 7200,
            end_epoch: day_start + 7260,
            steps: 5,
        };

        let sessions = session_from_raw(None, &[morning, evening]);
        assert_eq!(make_day_minutes(&sessions, day_start), 2);
    }

    #[test]
    async fn day_totals_split_session_across_midnight() {
        let midnight = jiff::civil::date(2026, 8, 9)
            .at(0, 0, 0, 0)
            .to_zoned(habits_stepper::time::TIMEZONE)
            .unwrap();
        let day_start = midnight.timestamp().as_second() as u32;

        let crossing = Session {
            start_epoch: day_start - 1200,
            end_epoch: day_start + 1200,
            steps: 40,
        };

        let sessions = session_from_raw(None, &[crossing]);
        assert_eq!(make_day_minutes(&sessions, day_start - 86400), 20);
        assert_eq!(make_day_minutes(&sessions, day_start), 20);
    }
}
