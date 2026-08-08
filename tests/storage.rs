#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(test)]
#[embedded_test::tests(executor = esp_rtos::embassy::Executor::new())]
mod tests {
    use core::ops::Range;

    use defmt::assert_eq;
    use habits_stepper::sessions::storage::{FlashRing, SLOT_COUNT};

    const SECTOR_SLOT_COUNT: u16 = 256;

    #[init]
    fn init() -> esp_hal::peripherals::FLASH<'static> {
        let peripherals = esp_hal::init(esp_hal::Config::default());
        let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
        let sw_interrupt =
            esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
        esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);
        rtt_target::rtt_init_defmt!();
        peripherals.FLASH
    }

    #[test]
    #[timeout(5)]
    async fn preserves_sessions_across_sector_rotations(
        flash: esp_hal::peripherals::FLASH<'static>,
    ) {
        let mut ring = create_erased_ring(flash);

        assert_eq!(
            write_sessions_with_step_count(&mut ring, 0..SECTOR_SLOT_COUNT, 1),
            0
        );
        assert_count(&mut ring, SECTOR_SLOT_COUNT);
        assert_session_step_counts(&mut ring, |_| 1);

        assert_eq!(
            write_sessions_with_step_count(&mut ring, SECTOR_SLOT_COUNT..SECTOR_SLOT_COUNT + 1, 2,),
            0
        );
        assert_count(&mut ring, SECTOR_SLOT_COUNT + 1);
        assert_session_step_counts(
            &mut ring,
            |index| {
                if index < SECTOR_SLOT_COUNT { 1 } else { 2 }
            },
        );

        assert_eq!(
            write_sessions_with_step_count(&mut ring, SECTOR_SLOT_COUNT + 1..SLOT_COUNT - 1, 2,),
            0
        );
        assert_count(&mut ring, SLOT_COUNT - 1);
        assert_session_step_counts(
            &mut ring,
            |index| {
                if index < SECTOR_SLOT_COUNT { 1 } else { 2 }
            },
        );

        assert_eq!(
            write_sessions_with_step_count(&mut ring, SLOT_COUNT - 1..SLOT_COUNT, 2),
            SECTOR_SLOT_COUNT
        );
        assert_count(&mut ring, SECTOR_SLOT_COUNT);
        assert_session_step_counts(&mut ring, |_| 2);

        assert_eq!(
            write_sessions_with_step_count(&mut ring, SLOT_COUNT..SLOT_COUNT + 1, 3),
            0
        );
        assert_count(&mut ring, SECTOR_SLOT_COUNT + 1);
        assert_session_step_counts(
            &mut ring,
            |index| {
                if index < SECTOR_SLOT_COUNT { 2 } else { 3 }
            },
        );

        ring.erase_all();
    }

    #[test]
    #[timeout(5)]
    async fn preserves_sessions_across_sector_rotations_after_reinitialization(
        flash: esp_hal::peripherals::FLASH<'static>,
    ) {
        let mut ring = create_erased_ring(flash);

        assert_eq!(
            write_sessions_with_step_count(&mut ring, 0..SECTOR_SLOT_COUNT, 1),
            0
        );
        assert_eq!(
            write_sessions_with_step_count(&mut ring, SECTOR_SLOT_COUNT..SECTOR_SLOT_COUNT + 1, 2,),
            0
        );
        assert_count(&mut ring, SECTOR_SLOT_COUNT + 1);

        ring.init();

        assert_count(&mut ring, SECTOR_SLOT_COUNT + 1);
        assert_session_step_counts(
            &mut ring,
            |index| {
                if index < SECTOR_SLOT_COUNT { 1 } else { 2 }
            },
        );

        assert_eq!(
            write_sessions_with_step_count(&mut ring, SECTOR_SLOT_COUNT + 1..SLOT_COUNT - 1, 2,),
            0
        );
        assert_count(&mut ring, SLOT_COUNT - 1);
        assert_session_step_counts(
            &mut ring,
            |index| {
                if index < SECTOR_SLOT_COUNT { 1 } else { 2 }
            },
        );

        assert_eq!(
            write_sessions_with_step_count(&mut ring, SLOT_COUNT - 1..SLOT_COUNT, 2),
            SECTOR_SLOT_COUNT
        );
        assert_count(&mut ring, SECTOR_SLOT_COUNT);
        assert_session_step_counts(&mut ring, |_| 2);

        assert_eq!(
            write_sessions_with_step_count(&mut ring, SLOT_COUNT..SLOT_COUNT + 1, 3),
            0
        );
        assert_count(&mut ring, SECTOR_SLOT_COUNT + 1);
        assert_session_step_counts(
            &mut ring,
            |index| {
                if index < SECTOR_SLOT_COUNT { 2 } else { 3 }
            },
        );

        ring.erase_all();
    }

    fn create_erased_ring(flash: esp_hal::peripherals::FLASH<'static>) -> FlashRing<'static> {
        let mut ring = FlashRing::new(flash);
        ring.erase_all();
        ring
    }

    fn write_sessions_with_step_count(
        ring: &mut FlashRing<'static>,
        session_indexes: Range<u16>,
        step_count: u32,
    ) -> u16 {
        let mut evicted = 0;
        for session_index in session_indexes {
            evicted +=
                ring.write_session(session_index as u32, session_index as u32 + 1, step_count);
        }
        evicted
    }

    fn assert_count(ring: &mut FlashRing<'static>, expected: u16) {
        assert_eq!(ring.count(), expected, "session count");
    }

    fn assert_session_step_counts(
        ring: &mut FlashRing<'static>,
        expected_step_count: impl Fn(u16) -> u32,
    ) {
        for (session_index, session) in ring.sessions().enumerate() {
            assert_eq!(
                session.steps,
                expected_step_count(session_index as u16),
                "session {} step count",
                session_index
            );
        }
    }
}
