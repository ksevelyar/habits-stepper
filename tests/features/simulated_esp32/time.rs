use core::net::{IpAddr, Ipv4Addr};

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use esp_hal::rtc_cntl::Rtc;
use habits_stepper::application::Configuration;

use super::wifi::{CONFIGURATION_READY, NETWORK_REQUESTS, NETWORK_RESPONSE, NetworkRequest};

pub use habits_stepper::time::{run_sleep_task, seed_from_rtc};

pub struct TimeNetwork {
    pub epoch_microseconds: u64,
}

impl habits_stepper::time::TimeNetwork for TimeNetwork {
    async fn wait_for_configuration(&mut self) {
        CONFIGURATION_READY.wait().await;
    }

    async fn resolve_server(&mut self) -> Option<IpAddr> {
        NETWORK_REQUESTS.send(NetworkRequest::Dns).await;
        NETWORK_RESPONSE.wait().await;
        Some(IpAddr::V4(Ipv4Addr::LOCALHOST))
    }

    async fn request_time(&mut self, _address: IpAddr, _current_time_us: u64) -> Option<u64> {
        NETWORK_REQUESTS.send(NetworkRequest::Ntp).await;
        NETWORK_RESPONSE.wait().await;
        Some(self.epoch_microseconds)
    }
}

#[embassy_executor::task]
pub async fn run_sync_time_task(
    mut network: TimeNetwork,
    rtc: &'static Mutex<CriticalSectionRawMutex, Rtc<'static>>,
    configuration: &'static Configuration,
) {
    habits_stepper::time::synchronize_with_network(&mut network, rtc, configuration).await;
}
