use core::net::{IpAddr, SocketAddr};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
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
use static_cell::{ConstStaticCell, StaticCell};

include!(concat!(env!("OUT_DIR"), "/timezone.rs"));

use sntpc::{NtpContext, NtpTimestampGenerator, get_time};
use sntpc_net_embassy::UdpSocketWrapper;

const NTP_SERVER: &str = "pool.ntp.org";
const MIN_VALID_EPOCH: u32 = 1_700_000_000;
const USEC_IN_SEC: u64 = 1_000_000;
const INACTIVITY: Duration = Duration::from_secs(120);

pub static TIME_SYNCHRONIZED: AtomicBool = AtomicBool::new(false);
static EPOCH_BASE: AtomicU32 = AtomicU32::new(0);
static INSTANT_BASE: AtomicU32 = AtomicU32::new(0);
static INSTANT_MILLIS_BASE: AtomicU32 = AtomicU32::new(0);

pub static RTC: StaticCell<Mutex<CriticalSectionRawMutex, Rtc<'static>>> = StaticCell::new();

defmt::timestamp!(
    "{}",
    LocalTime {
        zoned: get_local_time(),
        tenth_of_second: calculate_tenth_of_second(),
    }
);

pub fn get_epoch_seconds() -> Option<u32> {
    let base = EPOCH_BASE.load(Ordering::Acquire);
    if base == 0 {
        return None;
    }
    let now = Instant::now().as_secs();
    let ibase = u64::from(INSTANT_BASE.load(Ordering::Relaxed));
    let delta = now.saturating_sub(ibase) as u32;
    Some(base + delta)
}

pub fn calculate_week_range(now: u32) -> core::ops::Range<u32> {
    let timestamp =
        jiff::Timestamp::new(i64::from(now), 0).expect("epoch fits in a Jiff timestamp");
    let local_time = timestamp.to_zoned(TIMEZONE);
    let days_since_monday = local_time.weekday().since(Weekday::Monday);
    let monday = local_time
        .saturating_sub(jiff::Span::new().days(i64::from(days_since_monday)))
        .start_of_day()
        .expect("Monday has a start of day");
    let next_monday = monday
        .saturating_add(jiff::Span::new().days(7))
        .start_of_day()
        .expect("next Monday has a start of day");
    let start = u32::try_from(monday.timestamp().as_second()).expect("week start fits in an epoch");
    let end =
        u32::try_from(next_monday.timestamp().as_second()).expect("week end fits in an epoch");

    start..end
}

pub fn calculate_day_start(now: u32) -> u32 {
    let timestamp = jiff::Timestamp::new(i64::from(now), 0)
        .expect("u32 epoch seconds are within Jiff's timestamp range");
    let midnight = timestamp.to_zoned(TIMEZONE).start_of_day().unwrap();
    midnight.timestamp().as_second() as u32
}

pub fn calculate_next_day_start(day_start: u32) -> u32 {
    let timestamp = jiff::Timestamp::new(i64::from(day_start), 0)
        .expect("u32 epoch seconds are within Jiff's timestamp range");
    let next = timestamp
        .to_zoned(TIMEZONE)
        .saturating_add(jiff::Span::new().days(1));
    next.start_of_day().unwrap().timestamp().as_second() as u32
}

pub fn calculate_previous_day_start(now: u32) -> u32 {
    let timestamp = jiff::Timestamp::new(i64::from(now), 0)
        .expect("u32 epoch seconds are within Jiff's timestamp range");
    let yesterday = timestamp
        .to_zoned(TIMEZONE)
        .saturating_sub(jiff::Span::new().days(1));
    yesterday.start_of_day().unwrap().timestamp().as_second() as u32
}

pub fn format_date_bytes(now: u32) -> [u8; 10] {
    let timestamp = jiff::Timestamp::new(i64::from(now), 0)
        .expect("u32 epoch seconds are within Jiff's timestamp range");
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

fn calculate_tenth_of_second() -> u8 {
    if EPOCH_BASE.load(Ordering::Acquire) == 0 {
        return 0;
    }
    let now_millis = Instant::now().as_millis() as u32;
    let base_millis = INSTANT_MILLIS_BASE.load(Ordering::Relaxed);
    let subsec_millis = now_millis.wrapping_sub(base_millis) % 1000;
    (subsec_millis / 100) as u8
}

struct LocalTime {
    zoned: jiff::Zoned,
    tenth_of_second: u8,
}

impl defmt::Format for LocalTime {
    fn format(&self, formatter: defmt::Formatter) {
        defmt::write!(
            formatter,
            "{=i8:02}:{=i8:02}:{=i8:02}.{=u8}",
            self.zoned.hour(),
            self.zoned.minute(),
            self.zoned.second(),
            self.tenth_of_second,
        );
    }
}

fn get_local_time() -> jiff::Zoned {
    let epoch_seconds = get_epoch_seconds().unwrap_or(0);
    let timestamp = jiff::Timestamp::new(i64::from(epoch_seconds), 0)
        .expect("epoch seconds fit in a timestamp");

    timestamp.to_zoned(crate::time::TIMEZONE)
}

fn set_epoch(epoch_seconds: u32) {
    INSTANT_BASE.store(Instant::now().as_secs() as u32, Ordering::Relaxed);
    INSTANT_MILLIS_BASE.store(Instant::now().as_millis() as u32, Ordering::Relaxed);
    EPOCH_BASE.store(epoch_seconds, Ordering::Release);
}

pub fn seed_from_rtc(rtc: &Rtc) {
    let us = rtc.current_time_us();
    let secs = (us / USEC_IN_SEC) as u32;
    if secs > MIN_VALID_EPOCH {
        set_epoch(secs);
        let ts = Timestamp::new(i64::from(secs), 0)
            .expect("u32 epoch seconds are within Jiff's timestamp range");
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

    if let Some(epoch) = get_epoch_seconds() {
        let ts = Timestamp::new(i64::from(epoch), 0)
            .expect("u32 epoch seconds are within Jiff's timestamp range");
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
    address: IpAddr,
    stack: &Stack<'static>,
    rx_meta: &mut [PacketMetadata; 16],
    rx_buffer: &mut [u8; 4096],
    tx_meta: &mut [PacketMetadata; 16],
    tx_buffer: &mut [u8; 4096],
) -> Option<u64> {
    let socket = {
        let mut s = UdpSocket::new(*stack, rx_meta, rx_buffer, tx_meta, tx_buffer);
        s.bind(0).unwrap();
        UdpSocketWrapper::new(s)
    };

    let result = get_time(
        SocketAddr::from((address, 123)),
        &socket,
        NtpContext::new(NtpTs { current_time_us }),
    )
    .await;

    match result {
        Ok(time) => {
            let epoch_us = (u64::from(time.sec()) * USEC_IN_SEC)
                + ((u64::from(time.sec_fraction()) * USEC_IN_SEC) >> 32);

            Some(epoch_us)
        }
        Err(_e) => {
            error!("time: NTP failed");
            None
        }
    }
}

pub trait TimeNetwork {
    fn wait_for_configuration(&mut self) -> impl core::future::Future<Output = ()>;
    fn resolve_server(&mut self) -> impl core::future::Future<Output = Option<IpAddr>>;
    fn request_time(
        &mut self,
        address: IpAddr,
        current_time_us: u64,
    ) -> impl core::future::Future<Output = Option<u64>>;
}

struct HardwareTimeNetwork {
    stack: Stack<'static>,
    receive_buffer: &'static mut [u8; 4096],
    transmit_buffer: &'static mut [u8; 4096],
    receive_metadata: &'static mut [PacketMetadata; 16],
    transmit_metadata: &'static mut [PacketMetadata; 16],
}

impl TimeNetwork for HardwareTimeNetwork {
    async fn wait_for_configuration(&mut self) {
        self.stack.wait_config_up().await;
    }

    async fn resolve_server(&mut self) -> Option<IpAddr> {
        match self.stack.dns_query(NTP_SERVER, DnsQueryType::A).await {
            Ok(addresses) => addresses.first().copied().map(Into::into),
            Err(_) => None,
        }
    }

    async fn request_time(&mut self, address: IpAddr, current_time_us: u64) -> Option<u64> {
        sync_with_ntp(
            current_time_us,
            address,
            &self.stack,
            self.receive_metadata,
            self.receive_buffer,
            self.transmit_metadata,
            self.transmit_buffer,
        )
        .await
    }
}

#[embassy_executor::task]
pub async fn run_sync_time_task(
    stack: Stack<'static>,
    rtc: &'static Mutex<CriticalSectionRawMutex, Rtc<'static>>,
    configuration: &'static crate::application::Configuration,
) {
    static RECEIVE_BUFFER: ConstStaticCell<[u8; 4096]> = ConstStaticCell::new([0; 4096]);
    static TRANSMIT_BUFFER: ConstStaticCell<[u8; 4096]> = ConstStaticCell::new([0; 4096]);
    static RECEIVE_METADATA: ConstStaticCell<[PacketMetadata; 16]> =
        ConstStaticCell::new([PacketMetadata::EMPTY; 16]);
    static TRANSMIT_METADATA: ConstStaticCell<[PacketMetadata; 16]> =
        ConstStaticCell::new([PacketMetadata::EMPTY; 16]);

    let mut network = HardwareTimeNetwork {
        stack,
        receive_buffer: RECEIVE_BUFFER.take(),
        transmit_buffer: TRANSMIT_BUFFER.take(),
        receive_metadata: RECEIVE_METADATA.take(),
        transmit_metadata: TRANSMIT_METADATA.take(),
    };
    synchronize_with_network(&mut network, rtc, configuration).await;
}

pub async fn synchronize_with_network<Network: TimeNetwork>(
    network: &mut Network,
    rtc: &Mutex<CriticalSectionRawMutex, Rtc<'static>>,
    configuration: &crate::application::Configuration,
) {
    network.wait_for_configuration().await;

    let mut failed_attempts: u32 = 0;

    loop {
        crate::display::show_startup(crate::display::StartupStage::Dns).await;
        let address = with_timeout(Duration::from_secs(30), network.resolve_server()).await;
        let result = if let Ok(Some(address)) = address {
            crate::display::show_startup(crate::display::StartupStage::Ntp).await;
            let current_time_us = rtc.lock().await.current_time_us();
            with_timeout(
                Duration::from_secs(30),
                network.request_time(address, current_time_us),
            )
            .await
        } else {
            Ok(None)
        };

        if let Ok(Some(epoch_us)) = result {
            let new_epoch = (epoch_us / USEC_IN_SEC) as u32;
            let correction = get_epoch_seconds().map(|old| i64::from(new_epoch) - i64::from(old));

            rtc.lock().await.set_current_time_us(epoch_us);
            set_epoch(new_epoch);
            log_sync(correction);
            TIME_SYNCHRONIZED.store(true, Ordering::Release);
            crate::user_input::USER_INPUT_CHANNEL
                .send(crate::user_input::GpioEvent::TimeSynchronized)
                .await;

            break;
        }
        failed_attempts += 1;
        warn!("time: NTP sync attempt {} failed", failed_attempts);
        Timer::after(Duration::from_secs(10)).await;
    }

    info!("time: NTP sync complete");
    if !configuration.is_backend_configured() {
        crate::wifi::STOP_WIFI.signal(());
    }
}

#[embassy_executor::task]
pub async fn run_sleep_task(rtc: &'static Mutex<CriticalSectionRawMutex, Rtc<'static>>) {
    loop {
        info!("time: waiting {}s for inactivity", INACTIVITY.as_secs());
        if with_timeout(INACTIVITY, ACTIVITY.wait()).await.is_err() {
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
