use habits_stepper::user_input::{ACTIVITY, GpioEvent, USER_INPUT_CHANNEL};

pub async fn take_step() {
    ACTIVITY.signal(());
    USER_INPUT_CHANNEL.send(GpioEvent::StepDetected).await;
}

pub async fn press_history() {
    ACTIVITY.signal(());
    USER_INPUT_CHANNEL.send(GpioEvent::HistoryPressed).await;
}

pub async fn release_history() {
    ACTIVITY.signal(());
    USER_INPUT_CHANNEL.send(GpioEvent::HistoryReleased).await;
}
