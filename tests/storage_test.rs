//! NOTE: storage ring-buffer tests (two-sector design).
//!
//! ```sh
//! cargo test --test storage_test
//! ```

#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(test)]
#[embedded_test::tests(executor = esp_rtos::embassy::Executor::new())]
mod tests {
    use defmt::{assert_eq, info};
    use habits_stepper::sessions::storage::FlashRing;

    #[init]
    fn init() {
        let peripherals = esp_hal::init(esp_hal::Config::default());
        let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
        let sw_interrupt =
            esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
        esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);
        rtt_target::rtt_init_defmt!();
    }

    // NOTE: Fill one sector (256 slots), cross into second, then wrap around.
    #[test]
    async fn two_sector_ring() {
        info!("=== two-sector ring test ===");

        let flash = unsafe { esp_hal::peripherals::FLASH::steal() };
        let mut ring = FlashRing::new(flash);
        ring.erase_all();

        // NOTE: Step 1 — fill sector 0 with 256 sessions (steps=1)
        info!("--- step 1: write 256 sessions (steps=1) ---");
        for i in 0..256u16 {
            ring.write_session(i as u32, i as u32 + 1, 1);
        }

        let (n, h) = (ring.count(), ring.head());
        assert_eq!(n, 256, "count after filling sector 0");
        assert_eq!(h, 256, "head after filling sector 0");

        for i in 0..256u16 {
            let (_, _, steps, _) = ring.slot_at(i);
            assert_eq!(steps, 1, "slot[{}] steps = 1", i);
        }
        assert!(!ring.is_occupied(256), "slot 256 should be empty");
        info!("step 1 OK");

        // NOTE: Step 2 — cross into sector 1, write 1 session (steps=2)
        info!("--- step 2: write session 256 (steps=2) ---");
        ring.write_session(256, 257, 2);

        let (n, h) = (ring.count(), ring.head());
        assert_eq!(n, 257, "count after crossing");
        assert_eq!(h, 257, "head after crossing");

        {
            let (_, _, steps, _) = ring.slot_at(256);
            assert_eq!(steps, 2, "slot[256] steps = 2");
        }
        // first 256 should still be steps=1
        for i in 0..256u16 {
            let (_, _, steps, _) = ring.slot_at(i);
            assert_eq!(steps, 1, "slot[{}] still steps = 1 after cross", i);
        }
        info!("step 2 OK");

        // --------------------------------------------------------------------
        // Step 3 — fill sector 1 (255 more, total 512), verify archive intact
        // --------------------------------------------------------------------
        info!("--- step 3: fill sector 1 with 255 more sessions (steps=2) ---");
        for i in 257..512u16 {
            ring.write_session(i as u32, i as u32 + 1, 2);
        }

        let (n, h) = (ring.count(), ring.head());
        assert_eq!(n, 512, "count = 512 after filling both sectors");
        assert_eq!(h, 512, "head = 512");

        // sector 0 archive (steps=1) still intact
        for i in 0..256u16 {
            let (_, _, steps, _) = ring.slot_at(i);
            assert_eq!(steps, 1, "slot[{}] archive steps = 1", i);
        }
        // sector 1 (steps=2) intact
        for i in 256..512u16 {
            let (_, _, steps, _) = ring.slot_at(i);
            assert_eq!(steps, 2, "slot[{}] steps = 2", i);
        }
        info!("step 3 OK");

        // --------------------------------------------------------------------
        // Step 4 — write session 512 (steps=3) → wraps, erases sector 0
        // --------------------------------------------------------------------
        info!("--- step 4: write session 512 (steps=3, triggers wrap) ---");
        ring.write_session(512, 513, 3);

        let (n, h) = (ring.count(), ring.head());
        assert_eq!(n, 257, "count after wrap = 257");
        assert_eq!(h, 1, "head after wrap = 1");

        // slot 0 has the new session (steps=3)
        {
            let (_, _, steps, _) = ring.slot_at(0);
            assert_eq!(steps, 3, "slot[0] steps = 3");
        }
        // sector 1 archive (slots 256..511) steps=2 still intact
        for i in 256..512u16 {
            let (_, _, steps, _) = ring.slot_at(i);
            assert_eq!(steps, 2, "slot[{}] archive steps = 2 after wrap", i);
        }
        // slots 1..255 are now empty (sector 0 was erased, only slot 0 rewritten)
        for i in 1..256u16 {
            assert!(
                !ring.is_occupied(i),
                "slot[{}] should be empty after wrap",
                i
            );
        }

        info!("=== all steps PASSED, erasing all ===");
        ring.erase_all();
    }
}
