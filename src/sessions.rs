use esp_idf_svc::nvs::{EspKeyValueStorage, EspNvs, NvsDefault};
use log::info;
use postcard::{from_bytes, to_slice};

const SESSION_TIMEOUT_MS: u64 = 60 * 1000;
const MAX_SESSIONS: usize = 1024;
const SESSIONS_KEY: &str = "sessions";
const HEAD_KEY: &str = "head";
const COUNT_KEY: &str = "count";

const HEAD_BUF_SIZE: usize = 4;
const COUNT_BUF_SIZE: usize = 4;
const SESSIONS_BUF_SIZE: usize = 8192;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Session {
    pub start_time: u64,
    pub end_time: u64,
    pub steps: u32,
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

        let head = match storage.get_raw(HEAD_KEY, &mut head_buf) {
            Ok(Some(slice)) => from_bytes(slice).unwrap_or(0),
            _ => 0,
        };
        let count = match storage.get_raw(COUNT_KEY, &mut count_buf) {
            Ok(Some(slice)) => from_bytes(slice).unwrap_or(0),
            _ => 0,
        };
        (head, count)
    }

    fn save_metadata(&mut self) {
        let mut head_buf = [0u8; HEAD_BUF_SIZE];
        let mut count_buf = [0u8; COUNT_BUF_SIZE];

        let head_bytes = to_slice(&self.head, &mut head_buf).unwrap();
        let count_bytes = to_slice(&self.count, &mut count_buf).unwrap();
        let _ = self.storage.set_raw(HEAD_KEY, head_bytes);
        let _ = self.storage.set_raw(COUNT_KEY, count_bytes);
    }

    fn load_ring(&self) -> Vec<Session> {
        let mut buf = vec![0u8; SESSIONS_BUF_SIZE];
        match self.storage.get_raw(SESSIONS_KEY, &mut buf) {
            Ok(Some(slice)) => from_bytes::<SessionList>(slice)
                .map(|l| l.0)
                .unwrap_or_default(),
            _ => Vec::new(),
        }
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
                session.steps += 1;
                session.end_time = now;
                info!("step {} at {}", session.steps, now);
            }
            _ => {
                info!(
                    "session started at {}, timeout = {}",
                    now,
                    SESSION_TIMEOUT_MS / 1000
                );
                self.current = Some(Session {
                    start_time: now,
                    end_time: now,
                    steps: 1,
                });
            }
        }
    }

    pub fn tick(&mut self, now: u64) {
        if let Some(session) = self.current.take() {
            if now.saturating_sub(session.end_time) >= SESSION_TIMEOUT_MS {
                let duration = session.end_time - session.start_time;
                info!(
                    "session ended: {} steps, {}ms, head={}, count={}",
                    session.steps, duration, self.head, self.count
                );
                let mut ring = vec![None; MAX_SESSIONS];
                let existing = self.load_ring();
                for (i, s) in existing.iter().enumerate().take(MAX_SESSIONS) {
                    ring[i] = Some(s.clone());
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

    pub fn today_minutes(&self, now: u64) -> u32 {
        let today_start = Self::today_start(now);
        let mut total: u32 = 0;

        if let Some(session) = &self.current
            && session.start_time >= today_start
        {
            total += ((session.end_time - session.start_time) / 60000) as u32;
        }

        let ring = self.load_ring();
        for session in &ring {
            if session.start_time >= today_start {
                total += ((session.end_time - session.start_time) / 60000) as u32;
            }
        }

        total
    }

    fn today_start(now: u64) -> u64 {
        let day_ms: u64 = 24 * 60 * 60 * 1000;
        (now / day_ms) * day_ms
    }
}

pub fn new_storage(
    partition: esp_idf_svc::nvs::EspDefaultNvsPartition,
) -> anyhow::Result<EspKeyValueStorage<NvsDefault>> {
    let nvs = EspNvs::new(partition, "stepper", true)?;
    Ok(EspKeyValueStorage::new(nvs))
}
