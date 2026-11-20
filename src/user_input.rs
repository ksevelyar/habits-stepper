use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Input;

const DEBOUNCE: Duration = Duration::from_millis(50);

#[derive(PartialEq)]
pub enum GpioEvent {
    StepDetected,
    HistoryPressed,
    HistoryReleased,
    TimeSynchronized,
}

pub static ACTIVITY: Signal<CriticalSectionRawMutex, ()> = Signal::new();
pub static USER_INPUT_CHANNEL: Channel<CriticalSectionRawMutex, GpioEvent, 3> = Channel::new();

#[embassy_executor::task]
pub async fn run_reed_task(mut reed: Input<'static>) {
    let mut previous_state = reed.is_low();
    loop {
        reed.wait_for_any_edge().await;
        Timer::after(DEBOUNCE).await;

        let current_state = reed.is_low();
        if current_state != previous_state {
            info!(
                "input: reed {}",
                if current_state { "closed" } else { "opened" }
            );
            ACTIVITY.signal(());
            USER_INPUT_CHANNEL.send(GpioEvent::StepDetected).await;
            previous_state = current_state;
        }
    }
}

#[embassy_executor::task]
pub async fn run_history_task(mut history: Input<'static>) {
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
