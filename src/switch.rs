use esp_idf_svc::hal::gpio::{Input, InputPin, PinDriver, Pull};
use esp_idf_svc::sys::EspError;

#[derive(Debug, Clone)]
pub struct SwitchConfig {
    pub debounce_ms: u64,
    pub pull: Pull,
}

impl Default for SwitchConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 50,
            pull: Pull::Up,
        }
    }
}

pub struct Switch<'d> {
    pin: PinDriver<'d, Input>,
    config: SwitchConfig,
    last_state: bool,
    last_trigger_ms: u64,
    active_low: bool,
}

impl<'d> Switch<'d> {
    pub fn new<P: InputPin + 'd>(
        pin: P,
        config: SwitchConfig,
        active_low: bool,
    ) -> Result<Self, EspError> {
        let pin_driver = PinDriver::input(pin, config.pull)?;
        let initial_state = pin_driver.is_high();

        Ok(Self {
            pin: pin_driver,
            config,
            last_state: initial_state,
            last_trigger_ms: 0,
            active_low,
        })
    }

    pub fn poll(&mut self, now_ms: u64) -> bool {
        let raw_state = if self.active_low {
            self.pin.is_low()
        } else {
            self.pin.is_high()
        };

        if raw_state != self.last_state {
            self.last_state = raw_state;

            if now_ms.saturating_sub(self.last_trigger_ms) > self.config.debounce_ms {
                self.last_trigger_ms = now_ms;
                return true;
            }
        } else {
            self.last_state = raw_state;
        }

        false
    }
}
