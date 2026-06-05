use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer, with_timeout};

use crate::storage::{FlashRing, SLOT_COUNT};
use crate::{DISPLAY_CHANNEL, GpioEvent, USER_INPUT_CHANNEL};

const MAX_SESSIONS: usize = 256;
const TICK_INTERVAL: Duration = Duration::from_secs(1);
const SYNC_INTERVAL: Duration = Duration::from_secs(60);

pub type FlashMutex = Mutex<CriticalSectionRawMutex, FlashRing<'static>>;

#[derive(Clone, Copy)]
struct Session {
    start_epoch: u32,
    end_epoch: u32,
    steps: u32,
}

struct Sessions {
    current: Option<Session>,
    ring: [Option<Session>; MAX_SESSIONS],
    head: usize,
    count: usize,
}

impl Sessions {
    fn new(flash: &mut FlashRing<'static>) -> Self {
        flash.init();

        let mut ring = [const { None }; MAX_SESSIONS];
        let mut loaded = 0;

        for i in 0..SLOT_COUNT {
            if flash.is_occupied(i) {
                let (start_epoch, end_epoch, steps, _flags) = flash.slot_at(i);
                ring[loaded] = Some(Session {
                    start_epoch,
                    end_epoch,
                    steps,
                });
                loaded += 1;
            }
        }

        Self {
            current: None,
            ring,
            head: loaded,
            count: loaded,
        }
    }

    fn trigger(&mut self, now: u32) {
        match &mut self.current {
            Some(session) => {
                session.steps += 1;
                session.end_epoch = now;
            }
            None => {
                self.current = Some(Session {
                    start_epoch: now,
                    end_epoch: now,
                    steps: 1,
                });
            }
        }
    }

    fn tick(&mut self, now: u32, flash: &mut FlashRing<'static>) {
        let timed_out = self
            .current
            .as_ref()
            .is_some_and(|s| now.saturating_sub(s.end_epoch) >= 60);

        if timed_out {
            let session = self.current.take().unwrap();
            info!(
                "session ended: {} steps, {}min duration",
                session.steps,
                (session.end_epoch - session.start_epoch) / 60
            );
            flash.write_session(session.start_epoch, session.end_epoch, session.steps);
            let idx = self.head;
            self.ring[idx] = Some(session);
            self.head = (self.head + 1) % MAX_SESSIONS;
            self.count = self.count.saturating_add(1).min(MAX_SESSIONS);
        }
    }
}

fn minutes_in_range(session: &Session, range_start: u32, range_end: u32) -> u32 {
    let overlap_start = session.start_epoch.max(range_start);
    let overlap_end = session.end_epoch.min(range_end);
    if overlap_end > overlap_start {
        (overlap_end - overlap_start) / 60
    } else {
        0
    }
}

#[derive(Clone)]
pub struct SessionUpdate {
    pub today_minutes: u32,
    pub week_minutes: u32,
    pub today_steps: u32,
}

#[derive(Clone)]
pub struct SessionHistory {
    pub week1_minutes: u32,
    pub week2_minutes: u32,
    pub week3_minutes: u32,
}

#[derive(Clone)]
pub enum SessionEvent {
    Update(SessionUpdate),
    History(SessionHistory),
}

fn make_session_update(
    sessions: &Sessions,
    now: u32,
    prev: Option<&SessionEvent>,
) -> Option<SessionEvent> {
    let day_seconds: u32 = 86400;
    let week_seconds: u32 = 604800;
    let today_start = now.saturating_sub(day_seconds);
    let week_start = now.saturating_sub(week_seconds);

    let mut today_minutes: u32 = 0;
    let mut week_minutes: u32 = 0;
    let mut today_steps: u32 = 0;

    for session in sessions
        .ring
        .iter()
        .flatten()
        .chain(sessions.current.as_ref())
    {
        let today_overlap_start = session.start_epoch.max(today_start);
        let today_overlap_end = session.end_epoch.min(now);
        if today_overlap_end >= today_overlap_start {
            today_minutes += (today_overlap_end - today_overlap_start) / 60;
            today_steps += session.steps;
        }
        let week_overlap_start = session.start_epoch.max(week_start);
        let week_overlap_end = session.end_epoch.min(now);
        if week_overlap_end >= week_overlap_start {
            week_minutes += (week_overlap_end - week_overlap_start) / 60;
        }
    }

    let changed = match prev {
        None => true,
        Some(SessionEvent::Update(update)) => {
            today_minutes != update.today_minutes
                || week_minutes != update.week_minutes
                || today_steps != update.today_steps
        }
        Some(SessionEvent::History(_)) => true,
    };

    changed.then_some({
        SessionEvent::Update(SessionUpdate {
            today_minutes,
            week_minutes,
            today_steps,
        })
    })
}

fn make_session_history(
    sessions: &Sessions,
    now: u32,
    prev: Option<&SessionEvent>,
) -> Option<SessionEvent> {
    let week_seconds: u32 = 604800;
    let week0_start = now.saturating_sub(week_seconds);
    let week1_start = week0_start.saturating_sub(week_seconds);
    let week2_start = week1_start.saturating_sub(week_seconds);
    let week3_start = week2_start.saturating_sub(week_seconds);

    let mut week1_minutes: u32 = 0;
    let mut week2_minutes: u32 = 0;
    let mut week3_minutes: u32 = 0;

    for session in sessions
        .ring
        .iter()
        .flatten()
        .chain(sessions.current.as_ref())
    {
        week1_minutes += minutes_in_range(session, week1_start, week0_start);
        week2_minutes += minutes_in_range(session, week2_start, week1_start);
        week3_minutes += minutes_in_range(session, week3_start, week2_start);
    }

    let changed = match prev {
        None | Some(SessionEvent::Update(_)) => true,
        Some(SessionEvent::History(_)) => false,
    };

    changed.then_some({
        SessionEvent::History(SessionHistory {
            week1_minutes,
            week2_minutes,
            week3_minutes,
        })
    })
}

#[embassy_executor::task]
pub async fn session_task(flash_mutex: &'static FlashMutex) {
    let mut sessions = {
        let mut flash = flash_mutex.lock().await;
        Sessions::new(&mut flash)
    };
    let mut prev_event: Option<SessionEvent> = None;

    loop {
        let result = with_timeout(TICK_INTERVAL, USER_INPUT_CHANNEL.receive()).await;
        let epoch = crate::time::epoch_secs();

        if let Some(now) = epoch {
            {
                let mut flash = flash_mutex.lock().await;
                sessions.tick(now, &mut flash);
            }

            match result {
                Ok(GpioEvent::StepDetected) => {
                    sessions.trigger(now);
                    if let Some(event) = make_session_update(&sessions, now, prev_event.as_ref()) {
                        prev_event = Some(event.clone());
                        DISPLAY_CHANNEL.send(event).await;
                    }
                }
                Ok(GpioEvent::HistoryPressed) => {
                    if let Some(event) = make_session_history(&sessions, now, prev_event.as_ref()) {
                        prev_event = Some(event.clone());
                        DISPLAY_CHANNEL.send(event).await;
                    }
                }
                Ok(GpioEvent::HistoryReleased) => {
                    if let Some(event) = make_session_update(&sessions, now, prev_event.as_ref()) {
                        prev_event = Some(event.clone());
                        DISPLAY_CHANNEL.send(event).await;
                    }
                }
                Err(_err) => {}
            }
        }
    }
}

#[embassy_executor::task]
pub async fn sync_task(_flash_mutex: &'static FlashMutex) {
    loop {
        // Stub: sync always returns false
        Timer::after(SYNC_INTERVAL).await;
    }
}
