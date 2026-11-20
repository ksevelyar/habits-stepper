use core::sync::atomic::{AtomicBool, Ordering};

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use habits_stepper::application::Configuration;
use habits_stepper::wifi::WifiConnection;

const NETWORK_DELAY: Duration = Duration::from_millis(100);

pub(super) static CONFIGURATION_READY: Signal<CriticalSectionRawMutex, ()> = Signal::new();
pub(super) static NETWORK_REQUESTS: Channel<CriticalSectionRawMutex, NetworkRequest, 2> =
    Channel::new();
pub(super) static NETWORK_RESPONSE: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub(super) enum NetworkRequest {
    Dns,
    Ntp,
}

pub struct WifiController {
    pub connected: &'static AtomicBool,
}

impl WifiConnection for WifiController {
    async fn connect(&mut self) -> bool {
        Timer::after(NETWORK_DELAY).await;
        self.connected.store(true, Ordering::Release);
        true
    }

    async fn wait_for_disconnect(&mut self) {
        core::future::pending::<()>().await;
    }
}

impl Drop for WifiController {
    fn drop(&mut self) {
        self.connected.store(false, Ordering::Release);
    }
}

#[embassy_executor::task]
pub async fn run_wifi_task(controller: WifiController, configuration: &'static Configuration) {
    habits_stepper::wifi::run_wifi_connection(controller, configuration).await;
}

#[embassy_executor::task]
pub async fn run_network_task() {
    habits_stepper::wifi::wait_for_network_start().await;
    Timer::after(NETWORK_DELAY).await;
    CONFIGURATION_READY.signal(());
    super::sync::CONFIGURATION_READY.signal(());
    loop {
        let _request = NETWORK_REQUESTS.receive().await;
        Timer::after(NETWORK_DELAY).await;
        NETWORK_RESPONSE.signal(());
    }
}
