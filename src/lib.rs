#![no_std]
#![feature(type_alias_impl_trait)]

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

defmt::timestamp!(
    "{=u8:02}:{=u8:02}:{=u8:02}",
    {
        jiff::Timestamp::new(crate::time::epoch_secs().unwrap_or(0) as i64, 0)
            .unwrap()
            .to_zoned(crate::time::TIMEZONE)
            .hour() as u8
    },
    {
        jiff::Timestamp::new(crate::time::epoch_secs().unwrap_or(0) as i64, 0)
            .unwrap()
            .to_zoned(crate::time::TIMEZONE)
            .minute() as u8
    },
    {
        jiff::Timestamp::new(crate::time::epoch_secs().unwrap_or(0) as i64, 0)
            .unwrap()
            .to_zoned(crate::time::TIMEZONE)
            .second() as u8
    },
);

pub mod display;
pub mod sessions;
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
