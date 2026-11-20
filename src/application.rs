#[derive(Clone, Copy)]
pub struct Configuration {
    pub ssid: &'static str,
    pub pass: &'static str,
    pub wifi_transmit_power: &'static str,
    pub metrics_url: Option<&'static str>,
    pub jwt_token: Option<&'static str>,
    pub chain_id: Option<&'static str>,
}

pub static CONFIGURATION: Configuration = Configuration::load_from_environment();

impl Configuration {
    pub fn is_backend_configured(&self) -> bool {
        self.metrics_url.is_some() && self.jwt_token.is_some() && self.chain_id.is_some()
    }

    pub const fn load_from_environment() -> Self {
        Self {
            ssid: env!("SSID"),
            pass: env!("PASS"),
            wifi_transmit_power: env!("WIFI_TRANSMIT_POWER"),
            metrics_url: option_env!("METRICS_URL"),
            jwt_token: option_env!("JWT_TOKEN"),
            chain_id: option_env!("CHAIN_ID"),
        }
    }
}
