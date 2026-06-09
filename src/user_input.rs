use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Input;

use crate::{GpioEvent, USER_INPUT_CHANNEL};

const DEBOUNCE: Duration = Duration::from_millis(50);

pub static ACTIVITY: Signal<CriticalSectionRawMutex, ()> = Signal::new();

#[embassy_executor::task]
pub async fn reed_task(mut reed: Input<'static>) {
    loop {
        reed.wait_for_falling_edge().await;
        Timer::after(DEBOUNCE).await;

        if reed.is_low() {
            info!("input: reed closed");
            ACTIVITY.signal(());
            USER_INPUT_CHANNEL.send(GpioEvent::StepDetected).await;
            reed.wait_for_high().await;
            Timer::after(DEBOUNCE).await;
        }
    }
}

#[embassy_executor::task]
pub async fn history_task(mut history: Input<'static>) {
    loop {
        history.wait_for_falling_edge().await;
        Timer::after(DEBOUNCE).await;

        if history.is_low() {
            info!("input: history pressed");
            ACTIVITY.signal(());
            USER_INPUT_CHANNEL.send(GpioEvent::HistoryPressed).await;
            history.wait_for_rising_edge().await;
            Timer::after(DEBOUNCE).await;

            if history.is_high() {
                info!("input: history released");
                ACTIVITY.signal(());
                USER_INPUT_CHANNEL.send(GpioEvent::HistoryReleased).await;
            }
        }
    }
}
