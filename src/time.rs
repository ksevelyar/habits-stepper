use core::net::{IpAddr, SocketAddr};
use core::sync::atomic::{AtomicU32, Ordering};
use embassy_time::with_timeout;
use esp_hal::gpio::RtcPinWithResistors;
use esp_hal::peripherals::GPIO;
use esp_hal::rtc_cntl::sleep::{RtcioWakeupSource, WakeupLevel};

use defmt::{error, info, warn};
use embassy_net::{
    Stack,
    dns::DnsQueryType,
    udp::{PacketMetadata, UdpSocket},
};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};

use crate::user_input::ACTIVITY;
use esp_hal::rtc_cntl::Rtc;
use jiff::Timestamp;
use jiff::civil::Weekday;
use static_cell::StaticCell;

include!(concat!(env!("OUT_DIR"), "/timezone.rs"));

use sntpc::{NtpContext, NtpTimestampGenerator, get_time};
use sntpc_net_embassy::UdpSocketWrapper;

const NTP_SERVER: &str = "pool.ntp.org";
const MIN_VALID_EPOCH: u32 = 1_700_000_000;
const USEC_IN_SEC: u64 = 1_000_000;
const INACTIVITY: Duration = Duration::from_secs(90);

static EPOCH_BASE: AtomicU32 = AtomicU32::new(0);
static INSTANT_BASE: AtomicU32 = AtomicU32::new(0);

pub static RTC: StaticCell<Mutex<CriticalSectionRawMutex, Rtc<'static>>> = StaticCell::new();

defmt::timestamp!(
    "{=u8:02}:{=u8:02}:{=u8:02}",
    { local_time().hour() as u8 },
    { local_time().minute() as u8 },
    { local_time().second() as u8 },
);

pub fn epoch_secs() -> Option<u32> {
    let base = EPOCH_BASE.load(Ordering::Acquire);
    if base == 0 {
        return None;
    }
    let now = Instant::now().as_secs();
    let ibase = INSTANT_BASE.load(Ordering::Relaxed) as u64;
    let delta = now.saturating_sub(ibase) as u32;
    Some(base + delta)
}

pub fn week_start_epoch(now: u32) -> u32 {
    let timestamp = jiff::Timestamp::new(now as i64, 0).unwrap();
    let zoned = timestamp.to_zoned(TIMEZONE);
    let since_monday = zoned.weekday().since(Weekday::Monday);
    let monday = zoned.saturating_sub(jiff::Span::new().days(since_monday as i64));
    let midnight = monday.saturating_sub(
        jiff::Span::new()
            .hours(monday.hour() as i64)
            .minutes(monday.minute() as i64)
            .seconds(monday.second() as i64),
    );
    midnight.timestamp().as_second() as u32
}

pub fn day_start_epoch(now: u32) -> u32 {
    let timestamp = jiff::Timestamp::new(now as i64, 0).unwrap();
    let midnight = timestamp.to_zoned(TIMEZONE).start_of_day().unwrap();
    midnight.timestamp().as_second() as u32
}

pub fn next_day_start_epoch(day_start: u32) -> u32 {
    let timestamp = jiff::Timestamp::new(day_start as i64, 0).unwrap();
    let next = timestamp
        .to_zoned(TIMEZONE)
        .saturating_add(jiff::Span::new().days(1));
    next.start_of_day().unwrap().timestamp().as_second() as u32
}

pub fn previous_day_start_epoch(now: u32) -> u32 {
    let timestamp = jiff::Timestamp::new(now as i64, 0).unwrap();
    let yesterday = timestamp
        .to_zoned(TIMEZONE)
        .saturating_sub(jiff::Span::new().days(1));
    yesterday.start_of_day().unwrap().timestamp().as_second() as u32
}

pub fn date_bytes(now: u32) -> [u8; 10] {
    let timestamp = jiff::Timestamp::new(now as i64, 0).unwrap();
    let zoned = timestamp.to_zoned(TIMEZONE);
    let mut buf = [0u8; 10];
    write_digits(&mut buf[0..4], zoned.year() as u32);
    buf[4] = b'-';
    write_digits(&mut buf[5..7], zoned.month() as u32);
    buf[7] = b'-';
    write_digits(&mut buf[8..10], zoned.day() as u32);
    buf
}

fn write_digits(buf: &mut [u8], mut value: u32) {
    for byte in buf.iter_mut().rev() {
        *byte = b'0' + (value % 10) as u8;
        value /= 10;
    }
}

fn local_time() -> jiff::Zoned {
    let epoch_secs = epoch_secs().unwrap_or(0);
    let timestamp = jiff::Timestamp::new(epoch_secs as i64, 0).unwrap();

    timestamp.to_zoned(crate::time::TIMEZONE)
}

fn set_epoch(epoch_secs: u32) {
    INSTANT_BASE.store(Instant::now().as_secs() as u32, Ordering::Relaxed);
    EPOCH_BASE.store(epoch_secs, Ordering::Release);
}

fn seed_from_rtc(rtc: &Rtc) {
    let us = rtc.current_time_us();
    let secs = (us / USEC_IN_SEC) as u32;
    if secs > MIN_VALID_EPOCH {
        set_epoch(secs);
        let ts = Timestamp::new(secs as i64, 0).unwrap();
        let z = ts.to_zoned(TIMEZONE);
        info!(
            "time: rtc seeded: {=str} {=i16}-{=i8:02}-{=i8:02} {=i8:02}:{=i8:02}:{=i8:02}",
            TIMEZONE.iana_name().unwrap(),
            z.year(),
            z.month(),
            z.day(),
            z.hour(),
            z.minute(),
            z.second()
        );
    }
}

fn log_sync(correction: Option<i64>) {
    let corr = correction.unwrap_or(0);
    if corr >= 0 {
        info!("time: synced (+{=i64}s)", corr);
    } else {
        info!("time: synced ({=i64}s)", corr);
    }

    if let Some(epoch) = epoch_secs() {
        let ts = Timestamp::new(epoch as i64, 0).unwrap();
        let z = ts.to_zoned(TIMEZONE);
        info!(
            "time: {=str} {=i16}-{=i8:02}-{=i8:02} {=i8:02}:{=i8:02}:{=i8:02}",
            TIMEZONE.iana_name().unwrap(),
            z.year(),
            z.month(),
            z.day(),
            z.hour(),
            z.minute(),
            z.second()
        );
    }
}

#[derive(Clone, Copy)]
struct NtpTs {
    current_time_us: u64,
}

impl NtpTimestampGenerator for NtpTs {
    fn init(&mut self) {}
    fn timestamp_sec(&self) -> u64 {
        self.current_time_us / USEC_IN_SEC
    }
    fn timestamp_subsec_micros(&self) -> u32 {
        (self.current_time_us % USEC_IN_SEC) as u32
    }
}

async fn sync_with_ntp(
    current_time_us: u64,
    stack: &Stack<'static>,
    rx_meta: &mut [PacketMetadata; 16],
    rx_buffer: &mut [u8; 4096],
    tx_meta: &mut [PacketMetadata; 16],
    tx_buffer: &mut [u8; 4096],
) -> Option<u64> {
    let ntp_addrs = match stack.dns_query(NTP_SERVER, DnsQueryType::A).await {
        Ok(addrs) if !addrs.is_empty() => addrs,
        _ => {
            error!("time: DNS failed");
            return None;
        }
    };

    let socket = {
        let mut s = UdpSocket::new(*stack, rx_meta, rx_buffer, tx_meta, tx_buffer);
        s.bind(0).unwrap();
        UdpSocketWrapper::new(s)
    };

    let addr: IpAddr = ntp_addrs[0].into();
    let result = get_time(
        SocketAddr::from((addr, 123)),
        &socket,
        NtpContext::new(NtpTs { current_time_us }),
    )
    .await;

    match result {
        Ok(time) => {
            let epoch_us = (time.sec() as u64 * USEC_IN_SEC)
                + ((time.sec_fraction() as u64 * USEC_IN_SEC) >> 32);

            Some(epoch_us)
        }
        Err(_e) => {
            error!("time: NTP failed");
            None
        }
    }
}

#[embassy_executor::task]
pub async fn ntp_task(
    stack: Stack<'static>,
    rtc: &'static Mutex<CriticalSectionRawMutex, Rtc<'static>>,
) {
    seed_from_rtc(&*rtc.lock().await);

    stack.wait_config_up().await;

    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 4096];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; 4096];

    let mut failed_attempts: u32 = 0;

    loop {
        let current_time_us = rtc.lock().await.current_time_us();

        let result = with_timeout(
            Duration::from_secs(30),
            sync_with_ntp(
                current_time_us,
                &stack,
                &mut rx_meta,
                &mut rx_buffer,
                &mut tx_meta,
                &mut tx_buffer,
            ),
        )
        .await;

        match result {
            Ok(Some(epoch_us)) => {
                let new_epoch = (epoch_us / USEC_IN_SEC) as u32;
                let correction = epoch_secs().map(|old| new_epoch as i64 - old as i64);

                rtc.lock().await.set_current_time_us(epoch_us);
                set_epoch(new_epoch);
                log_sync(correction);

                break;
            }
            Ok(None) | Err(_) => {
                failed_attempts += 1;
                warn!("time: NTP sync attempt {} failed", failed_attempts);
                Timer::after(Duration::from_secs(10)).await;
            }
        }
    }

    info!("time: NTP sync complete");
}

#[embassy_executor::task]
pub async fn sleep_task(rtc: &'static Mutex<CriticalSectionRawMutex, Rtc<'static>>) {
    loop {
        info!("time: waiting {}s for inactivity", INACTIVITY.as_secs());
        match with_timeout(INACTIVITY, ACTIVITY.wait()).await {
            Ok(()) => {
                info!("time: activity before timeout, resetting");
                continue;
            }
            Err(_) => {
                info!("time: user input timeout -> entering deep sleep");

                let reed_level = {
                    let bits = GPIO::regs().in_().read().bits();
                    if bits & (1 << 1) != 0 {
                        WakeupLevel::Low
                    } else {
                        WakeupLevel::High
                    }
                };

                // SAFETY: we are about to enter deep sleep, which resets the chip.
                // No other task can access these pins before the reset.
                let (mut gpio1, mut gpio2) = unsafe {
                    (
                        esp_hal::peripherals::GPIO1::steal(),
                        esp_hal::peripherals::GPIO2::steal(),
                    )
                };

                let mut wake_pins = [
                    (&mut gpio1 as &mut dyn RtcPinWithResistors, reed_level),
                    (&mut gpio2 as &mut dyn RtcPinWithResistors, WakeupLevel::Low),
                ];
                let wake = RtcioWakeupSource::new(&mut wake_pins);

                rtc.lock().await.sleep_deep(&[&wake]);
            }
        }
    }
}
