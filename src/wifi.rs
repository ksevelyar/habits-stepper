use defmt::{error, info, warn};
use embassy_net::{Runner, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::rng::Rng;
pub use esp_radio::wifi::Interface;
use static_cell::ConstStaticCell;

use esp_radio::wifi::{Config, ControllerConfig, WifiController, sta::StationConfig};

use crate::application::Configuration;

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

#[embassy_executor::task]
pub async fn connect(
    mut controller: WifiController<'static>,
    configuration: &'static Configuration,
) {
    let mut failed_attempts: u32 = 0;

    loop {
        info!("wifi: connecting to \"{}\"", configuration.ssid);

        match controller.connect_async().await {
            Ok(info) => {
                failed_attempts = 0;
                info!("wifi: connected to \"{}\"", info.ssid.as_str());

                wait_for_disconnect(&mut controller).await;
            }
            Err(err) => {
                failed_attempts += 1;
                warn!("wifi: connection attempt {} failed", failed_attempts);
                match err {
                    esp_radio::wifi::WifiError::Disconnected(info) => {
                        error!(
                            "wifi: disconnected: SSID: \"{}\", reason: {:?}, RSSI: {}",
                            info.ssid.as_str(),
                            info.reason,
                            info.rssi,
                        );
                    }
                    _ => {
                        error!("wifi: connection failed: {:?}", err);
                    }
                }
            }
        }

        Timer::after(Duration::from_millis(5000)).await;
    }
}

async fn wait_for_disconnect(controller: &mut WifiController<'static>) {
    match controller.wait_for_disconnect_async().await {
        Ok(info) => {
            info!(
                "wifi: disconnected from \"{}\" (reason: {:?}, rssi: {})",
                info.ssid.as_str(),
                info.reason,
                info.rssi,
            );
        }
        Err(error) => {
            warn!("wifi: disconnect wait failed: {:?}", error);
        }
    }
}

#[embassy_executor::task]
pub async fn run_network(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await;
}
