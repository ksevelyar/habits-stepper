use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Input;

use crate::{GpioEvent, USER_INPUT_CHANNEL};

const POLL: Duration = Duration::from_millis(10);
const DEBOUNCE: Duration = Duration::from_millis(50);

pub static ACTIVITY: Signal<CriticalSectionRawMutex, ()> = Signal::new();

#[embassy_executor::task]
pub async fn task(reed: Input<'static>, history: Input<'static>) {
    let mut reed_closed = reed.is_low();
    let mut hist_closed = history.is_low();

    loop {
        Timer::after(POLL).await;

        let reed_now = reed.is_low();
        let hist_now = history.is_low();
        let reed_falling_edge = !reed_closed && reed_now;
        let hist_changed = hist_closed != hist_now;

        if reed_falling_edge {
            Timer::after(DEBOUNCE).await;
        }
        if reed_falling_edge && reed.is_low() {
            info!("user_input: reed closed -> signal activity");
            ACTIVITY.signal(());
            USER_INPUT_CHANNEL.send(GpioEvent::StepDetected).await;
            while reed.is_low() {
                Timer::after(POLL).await;
            }
            Timer::after(DEBOUNCE).await;
        }

        if hist_changed {
            Timer::after(DEBOUNCE).await;
        }
        if hist_changed && history.is_low() != hist_closed {
            let pressed = history.is_low();
            let event = if pressed {
                GpioEvent::HistoryPressed
            } else {
                GpioEvent::HistoryReleased
            };
            info!(
                "user_input: history {} -> signal activity",
                if pressed { "pressed" } else { "released" }
            );
            ACTIVITY.signal(());
            USER_INPUT_CHANNEL.send(event).await;
        }

        reed_closed = reed.is_low();
        hist_closed = history.is_low();
    }
}
