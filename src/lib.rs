#![no_std]
#![feature(type_alias_impl_trait)]

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

pub mod display;
pub mod sessions;
pub mod storage;
pub mod time;
pub mod user_input;
pub mod wifi;

#[derive(PartialEq)]
pub enum GpioEvent {
    StepDetected,
    HistoryPressed,
    HistoryReleased,
}

pub static USER_INPUT_CHANNEL: Channel<CriticalSectionRawMutex, GpioEvent, 3> = Channel::new();
pub static DISPLAY_CHANNEL: Channel<CriticalSectionRawMutex, sessions::SessionEvent, 3> =
    Channel::new();
