#![no_std]
#![cfg_attr(test, no_main)]

#[cfg(test)]
use panic_rtt_target as _;

pub mod application;
pub mod display;
pub mod sessions;
pub mod sync;
pub mod time;
pub mod user_input;
pub mod wifi;
