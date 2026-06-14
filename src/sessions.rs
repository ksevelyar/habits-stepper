pub mod storage;

use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer, with_timeout};

use crate::{DISPLAY_CHANNEL, GpioEvent, USER_INPUT_CHANNEL};
use storage::{FlashRing, SLOT_COUNT};

const MAX_SESSIONS: usize = SLOT_COUNT as usize;
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
        // TODO: add test to the case when ntp time jumps backward
        let timed_out = self.current.as_ref().is_some_and(|session| {
            if now < session.end_epoch {
                true
            } else {
                now - session.end_epoch >= 60
            }
        });

        if timed_out {
            let session = self.current.take().unwrap();
            info!(
                "session: ended: {} steps, {}min duration",
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
    pub week_minutes: u32,
    pub week_steps: u32,
}

#[derive(Clone)]
pub struct SessionHistory {
    pub prev_week_minutes: u32,
    pub prev_week_steps: u32,
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
    let week_seconds: u32 = 604800;
    let week_start = now.saturating_sub(week_seconds);

    let mut week_minutes: u32 = 0;
    let mut week_steps: u32 = 0;

    for session in sessions
        .ring
        .iter()
        .flatten()
        .chain(sessions.current.as_ref())
    {
        let week_overlap_start = session.start_epoch.max(week_start);
        let week_overlap_end = session.end_epoch.min(now);
        if week_overlap_end >= week_overlap_start {
            week_minutes += (week_overlap_end - week_overlap_start) / 60;
            week_steps += session.steps;
        }
    }

    let changed = match prev {
        None => true,
        Some(SessionEvent::Update(update)) => {
            week_minutes != update.week_minutes || week_steps != update.week_steps
        }
        Some(SessionEvent::History(_)) => true,
    };

    changed.then_some({
        SessionEvent::Update(SessionUpdate {
            week_minutes,
            week_steps,
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

    let mut prev_week_minutes: u32 = 0;
    let mut prev_week_steps: u32 = 0;

    for session in sessions
        .ring
        .iter()
        .flatten()
        .chain(sessions.current.as_ref())
    {
        prev_week_minutes += minutes_in_range(session, week1_start, week0_start);
        let overlap_start = session.start_epoch.max(week1_start);
        let overlap_end = session.end_epoch.min(week0_start);
        if overlap_end > overlap_start {
            prev_week_steps += session.steps;
        }
    }

    let changed = match prev {
        None | Some(SessionEvent::Update(_)) => true,
        Some(SessionEvent::History(_)) => false,
    };

    changed.then_some({
        SessionEvent::History(SessionHistory {
            prev_week_minutes,
            prev_week_steps,
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
