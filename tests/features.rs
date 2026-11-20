#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

#[embedded_test::setup]
fn initialize_debug_output() {
    rtt_target::rtt_init_defmt!();
}

mod features {
    mod sessions {
        mod backend_sync;
        mod permanent_storage;
        mod tracking;
    }
    mod simulated_esp32;
    mod time_sync;
}
