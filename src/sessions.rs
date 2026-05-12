use esp_idf_svc::nvs::{EspKeyValueStorage, EspNvs, NvsDefault};
use log::info;
use postcard::{from_bytes, to_slice};
use std::ops::Range;

const SESSION_TIMEOUT_MS: u64 = 60 * 1000;
const MAX_SESSIONS: usize = 1024;

const SESSIONS_KEY: &str = "sessions";
const HEAD_KEY: &str = "head";
const COUNT_KEY: &str = "count";

const HEAD_BUF_SIZE: usize = 4;
const COUNT_BUF_SIZE: usize = 4;
const SESSIONS_BUF_SIZE: usize = 8192;

const MINUTE_MS: u64 = 60_000;
const DAY_MS: u64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Session {
    pub time: Range<u64>,
    pub steps: u32,
}

impl Session {
    fn new(now: u64) -> Self {
        Self {
            time: now..now,
            steps: 1,
        }
    }

    fn touch(&mut self, now: u64) {
        self.steps += 1;
        self.time.end = now;
    }

    fn duration_minutes(&self) -> u32 {
        ((self.time.end - self.time.start) / MINUTE_MS) as u32
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SessionList(Vec<Session>);

pub struct Sessions {
    storage: EspKeyValueStorage<NvsDefault>,
    current: Option<Session>,
    head: u16,
    count: u16,
}

impl Sessions {
    pub fn new(storage: EspKeyValueStorage<NvsDefault>) -> Self {
        let (head, count) = Self::load_metadata(&storage);

        Self {
            storage,
            current: None,
            head,
            count,
        }
    }

    fn load_metadata(storage: &EspKeyValueStorage<NvsDefault>) -> (u16, u16) {
        let mut head_buf = [0u8; HEAD_BUF_SIZE];
        let mut count_buf = [0u8; COUNT_BUF_SIZE];

        let head = storage
            .get_raw(HEAD_KEY, &mut head_buf)
            .ok()
            .flatten()
            .and_then(|s| from_bytes(s).ok())
            .unwrap_or(0);

        let count = storage
            .get_raw(COUNT_KEY, &mut count_buf)
            .ok()
            .flatten()
            .and_then(|s| from_bytes(s).ok())
            .unwrap_or(0);

        (head, count)
    }

    fn save_metadata(&mut self) {
        let mut head_buf = [0u8; HEAD_BUF_SIZE];
        let mut count_buf = [0u8; COUNT_BUF_SIZE];

        let _ = self
            .storage
            .set_raw(HEAD_KEY, to_slice(&self.head, &mut head_buf).unwrap());

        let _ = self
            .storage
            .set_raw(COUNT_KEY, to_slice(&self.count, &mut count_buf).unwrap());
    }

    fn load_ring(&self) -> Vec<Session> {
        let mut buf = vec![0u8; SESSIONS_BUF_SIZE];

        self.storage
            .get_raw(SESSIONS_KEY, &mut buf)
            .ok()
            .flatten()
            .and_then(|slice| from_bytes::<SessionList>(slice).ok())
            .map(|l| l.0)
            .unwrap_or_default()
    }

    fn save_ring(&mut self, ring: &[Option<Session>]) {
        let sessions: Vec<Session> = ring.iter().filter_map(|s| s.clone()).collect();

        let mut buf = vec![0u8; SESSIONS_BUF_SIZE];

        if let Ok(slice) = to_slice(&SessionList(sessions), &mut buf) {
            let _ = self.storage.set_raw(SESSIONS_KEY, slice);
        }
    }

    pub fn trigger(&mut self, now: u64) {
        match &mut self.current {
            Some(session) => {
                session.touch(now);
                info!("step {} at {}", session.steps, now);
            }
            None => {
                info!(
                    "session started at {}, timeout={}s",
                    now,
                    SESSION_TIMEOUT_MS / 1000
                );
                self.current = Some(Session::new(now));
            }
        }
    }

    pub fn tick(&mut self, now: u64) {
        if let Some(session) = self.current.take() {
            if now.saturating_sub(session.time.end) >= SESSION_TIMEOUT_MS {
                info!(
                    "session ended: {} steps, {}min",
                    session.steps,
                    session.duration_minutes()
                );

                let mut ring = vec![None; MAX_SESSIONS];

                for (i, s) in self.load_ring().into_iter().enumerate() {
                    if i < MAX_SESSIONS {
                        ring[i] = Some(s);
                    }
                }

                let idx = self.head as usize;
                ring[idx] = Some(session);

                self.count = self.count.saturating_add(1).min(MAX_SESSIONS as u16);

                self.head = (self.head + 1) % MAX_SESSIONS as u16;

                self.save_ring(&ring);
                self.save_metadata();
            } else {
                self.current = Some(session);
            }
        }
    }

    fn overlap(a: &Range<u64>, b: &Range<u64>) -> Option<Range<u64>> {
        let start = a.start.max(b.start);
        let end = a.end.min(b.end);
        (end > start).then_some(start..end)
    }

    fn overlap_minutes(session: &Session, range: &Range<u64>) -> u32 {
        Self::overlap(&session.time, range)
            .map(|r| ((r.end - r.start) / MINUTE_MS) as u32)
            .unwrap_or(0)
    }

    fn all_sessions(&self) -> impl Iterator<Item = Session> {
        self.load_ring().into_iter().chain(self.current.clone())
    }

    pub fn today_minutes(&self, now: u64) -> u32 {
        let range = Self::day_start(now)..now;

        self.all_sessions()
            .map(|s| Self::overlap_minutes(&s, &range))
            .sum()
    }

    pub fn week_minutes(&self, now: u64) -> u32 {
        let range = Self::week_start(now)..now;

        self.all_sessions()
            .map(|s| Self::overlap_minutes(&s, &range))
            .sum()
    }

    fn day_start(now: u64) -> u64 {
        (now / DAY_MS) * DAY_MS
    }

    fn week_start(now: u64) -> u64 {
        let days_since_epoch = now / DAY_MS;

        // NOTE: Monday start, unix epoch started Thursday
        let weekday = (days_since_epoch + 3) % 7;

        let monday_days = days_since_epoch - weekday;
        monday_days * DAY_MS
    }
}

pub fn new_storage(
    partition: esp_idf_svc::nvs::EspDefaultNvsPartition,
) -> anyhow::Result<EspKeyValueStorage<NvsDefault>> {
    let nvs = EspNvs::new(partition, "stepper", true)?;
    Ok(EspKeyValueStorage::new(nvs))
}
