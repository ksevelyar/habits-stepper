use defmt::{error, info, warn};
use embassy_futures::select::select;
use embassy_net::{Runner, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::rng::Rng;
pub use esp_radio::wifi::Interface;
use static_cell::ConstStaticCell;

use esp_radio::wifi::{Config, ControllerConfig, WifiController, sta::StationConfig};

use crate::application::Configuration;
use core::sync::atomic::{AtomicBool, Ordering};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

static NETWORK_READY: Signal<CriticalSectionRawMutex, ()> = Signal::new();
pub static STOP_WIFI: Signal<CriticalSectionRawMutex, ()> = Signal::new();
pub static DHCP_STARTED: AtomicBool = AtomicBool::new(false);

extern crate alloc;

pub fn init(
    wifi: esp_hal::peripherals::WIFI<'static>,
    configuration: &Configuration,
) -> (
    WifiController<'static>,
    Stack<'static>,
    Runner<'static, Interface<'static>>,
) {
    static NETWORK_RESOURCES: ConstStaticCell<StackResources<6>> =
        ConstStaticCell::new(StackResources::new());

    let (mut controller, interfaces) = initialize_controller(wifi, configuration);
    configure_transmit_power(&mut controller, configuration);

    let config = embassy_net::Config::dhcpv4(embassy_net::DhcpConfig::default());
    let seed = u64::from(Rng::new().random()) << 32 | u64::from(Rng::new().random());

    let (stack, runner) =
        embassy_net::new(interfaces.station, config, NETWORK_RESOURCES.take(), seed);

    (controller, stack, runner)
}

fn initialize_controller(
    wifi: esp_hal::peripherals::WIFI<'static>,
    configuration: &Configuration,
) -> (
    WifiController<'static>,
    esp_radio::wifi::Interfaces<'static>,
) {
    let station_config = Config::Station(
        StationConfig::default()
            .with_ssid(configuration.ssid)
            .with_password(configuration.pass.into()),
    );

    esp_radio::wifi::new(
        wifi,
        ControllerConfig::default().with_initial_config(station_config),
    )
    .expect("Failed to initialize Wi-Fi controller")
}

fn configure_transmit_power(
    controller: &mut WifiController<'static>,
    configuration: &Configuration,
) {
    let max_tx_power_dbm: i8 = configuration
        .wifi_transmit_power
        .parse()
        .expect("WIFI_TRANSMIT_POWER must be a valid integer");
    controller
        // NOTE: Power unit is 0.25dBm, range is [8, 84] corresponding to 2dBm - 20dBm.
        .set_max_tx_power(max_tx_power_dbm * 4)
        .expect("Failed to set max TX power");
}

pub trait WifiConnection {
    fn connect(&mut self) -> impl core::future::Future<Output = bool>;
    fn wait_for_disconnect(&mut self) -> impl core::future::Future<Output = ()>;
}

impl WifiConnection for WifiController<'static> {
    async fn connect(&mut self) -> bool {
        match self.connect_async().await {
            Ok(info) => {
                info!("wifi: connected to \"{}\"", info.ssid.as_str());
                true
            }
            Err(error) => {
                if let esp_radio::wifi::WifiError::Disconnected(info) = error {
                    error!(
                        "wifi: disconnected: SSID: \"{}\", reason: {:?}, RSSI: {}",
                        info.ssid.as_str(),
                        info.reason,
                        info.rssi,
                    );
                } else {
                    error!("wifi: connection failed: {:?}", error);
                }
                false
            }
        }
    }

    async fn wait_for_disconnect(&mut self) {
        match self.wait_for_disconnect_async().await {
            Ok(info) => {
                info!(
                    "wifi: disconnected from \"{}\" (reason: {:?}, rssi: {})",
                    info.ssid.as_str(),
                    info.reason,
                    info.rssi,
                );
            }
            Err(error) => warn!("wifi: disconnect wait failed: {:?}", error),
        }
    }
}

pub async fn maintain_connection<Connection: WifiConnection>(
    controller: &mut Connection,
    configuration: &Configuration,
) {
    let mut failed_attempts: u32 = 0;
    loop {
        info!("wifi: connecting to \"{}\"", configuration.ssid);
        if controller.connect().await {
            failed_attempts = 0;
            if !DHCP_STARTED.load(Ordering::Acquire) {
                crate::display::show_startup(crate::display::StartupStage::Dhcp).await;
                DHCP_STARTED.store(true, Ordering::Release);
                NETWORK_READY.signal(());
            }
            controller.wait_for_disconnect().await;
        } else {
            failed_attempts += 1;
            warn!("wifi: connection attempt {} failed", failed_attempts);
        }
        Timer::after(Duration::from_millis(2000)).await;
    }
}

pub async fn run_wifi_connection<Connection: WifiConnection>(
    mut controller: Connection,
    configuration: &'static Configuration,
) {
    crate::display::show_startup(crate::display::StartupStage::Wifi).await;
    let connection = maintain_connection(&mut controller, configuration);
    let shutdown = async {
        STOP_WIFI.wait().await;
        info!("wifi: stopping after time synchronization");
    };
    select(connection, shutdown).await;
}

pub async fn wait_for_network_start() {
    NETWORK_READY.wait().await;
}

#[embassy_executor::task]
pub async fn run_wifi_task(
    controller: WifiController<'static>,
    configuration: &'static Configuration,
) {
    run_wifi_connection(controller, configuration).await;
}

#[embassy_executor::task]
pub async fn run_network_task(mut runner: Runner<'static, Interface<'static>>) {
    wait_for_network_start().await;
    runner.run().await;
}
