#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(test)]
#[embedded_test::tests(executor = esp_rtos::embassy::Executor::new())]
mod tests {
    use defmt::assert_eq;
    use habits_stepper::time::{date_bytes, day_start_epoch};

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
    async fn day_start_matches_local_midnight() {
        let noon = jiff::civil::date(2026, 8, 9)
            .at(14, 30, 0, 0)
            .to_zoned(habits_stepper::time::TIMEZONE)
            .unwrap();
        let now = noon.timestamp().as_second() as u32;

        let expected = jiff::civil::date(2026, 8, 9)
            .at(0, 0, 0, 0)
            .to_zoned(habits_stepper::time::TIMEZONE)
            .unwrap()
            .timestamp()
            .as_second() as u32;

        assert_eq!(day_start_epoch(now), expected);
    }

    #[test]
    async fn date_bytes_formats_iso_date() {
        let ts = jiff::civil::date(2026, 8, 9)
            .at(23, 45, 0, 0)
            .to_zoned(habits_stepper::time::TIMEZONE)
            .unwrap();
        let now = ts.timestamp().as_second() as u32;

        assert_eq!(date_bytes(now), *b"2026-08-09");
        assert_eq!(date_bytes(day_start_epoch(now)), *b"2026-08-09");
    }

    #[test]
    async fn date_bytes_after_midnight_is_next_day() {
        let ts = jiff::civil::date(2026, 8, 10)
            .at(0, 0, 30, 0)
            .to_zoned(habits_stepper::time::TIMEZONE)
            .unwrap();
        let now = ts.timestamp().as_second() as u32;

        assert_eq!(date_bytes(now), *b"2026-08-10");
    }
}
