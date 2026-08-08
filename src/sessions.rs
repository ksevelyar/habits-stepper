pub mod storage;

use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, with_timeout};
use heapless::Deque;

use crate::{DISPLAY_CHANNEL, GpioEvent, USER_INPUT_CHANNEL};
use storage::{FlashRing, SLOT_COUNT};

const MAX_SESSIONS: usize = SLOT_COUNT as usize;
const TICK_INTERVAL: Duration = Duration::from_secs(1);
const WEEK_SECONDS: u32 = 604800;

pub type FlashMutex = Mutex<CriticalSectionRawMutex, FlashRing<'static>>;

#[derive(Clone, Copy, Debug)]
pub struct Session {
    pub start_epoch: u32,
    pub end_epoch: u32,
    pub steps: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct DailyTotal {
    pub date: u32,
    pub minutes: u32,
}

pub struct Sessions {
    pub current: Option<Session>,
    pub history: Deque<Session, MAX_SESSIONS>,
}

impl Sessions {
    fn new(flash: &mut FlashRing<'static>) -> Self {
        flash.init();

        let mut history = Deque::new();

        for record in flash.sessions() {
            history
                .push_back(Session {
                    start_epoch: record.start_epoch,
                    end_epoch: record.end_epoch,
                    steps: record.steps,
                })
                .unwrap();
        }

        Self {
            current: None,
            history,
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

    fn tick(&mut self, now: u32, flash: &mut FlashRing<'static>) -> Option<Session> {
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
            let evicted =
                flash.write_session(session.start_epoch, session.end_epoch, session.steps);
            for _ in 0..evicted {
                self.history.pop_front();
            }
            self.history.push_back(session).unwrap();
            Some(session)
        } else {
            None
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

pub fn make_day_minutes(sessions: &Sessions, day_start: u32) -> u32 {
    let day_end = crate::time::next_day_start_epoch(day_start);
    let mut minutes: u32 = 0;

    for session in sessions.history.iter().chain(sessions.current.as_ref()) {
        minutes += minutes_in_range(session, day_start, day_end);
    }

    minutes
}

#[derive(Clone, Copy)]
pub struct WeekTotals {
    pub minutes: u32,
    pub steps: u32,
}

#[derive(Clone)]
pub enum SessionEvent {
    Update(WeekTotals),
    History(WeekTotals),
}

pub fn make_week_totals(sessions: &Sessions, now: u32) -> WeekTotals {
    let week_start = crate::time::week_start_epoch(now);
    let week_end = week_start + WEEK_SECONDS;
    let mut minutes: u32 = 0;
    let mut steps: u32 = 0;

    for session in sessions.history.iter().chain(sessions.current.as_ref()) {
        minutes += minutes_in_range(session, week_start, week_end);
        if session.end_epoch.min(week_end) > session.start_epoch.max(week_start) {
            steps += session.steps;
        }
    }

    WeekTotals { minutes, steps }
}

#[embassy_executor::task]
pub async fn session_task(flash_mutex: &'static FlashMutex) {
    let mut sessions = {
        let mut flash = flash_mutex.lock().await;
        Sessions::new(&mut flash)
    };

    loop {
        let result = with_timeout(TICK_INTERVAL, USER_INPUT_CHANNEL.receive()).await;
        let epoch = crate::time::epoch_secs();

        if let Some(now) = epoch {
            let completed = {
                let mut flash = flash_mutex.lock().await;
                sessions.tick(now, &mut flash)
            };

            if completed.is_some() {
                report_completed_session(&sessions, now);
            }

            match result {
                Ok(event @ (GpioEvent::StepDetected | GpioEvent::HistoryReleased)) => {
                    if matches!(event, GpioEvent::StepDetected) {
                        sessions.trigger(now);
                    }
                    let totals = make_week_totals(&sessions, now);
                    DISPLAY_CHANNEL.send(SessionEvent::Update(totals)).await;
                }
                Ok(GpioEvent::HistoryPressed) => {
                    let totals = make_week_totals(&sessions, now.saturating_sub(WEEK_SECONDS));
                    DISPLAY_CHANNEL.send(SessionEvent::History(totals)).await;
                }
                Err(_) => {}
            }
        }
    }
}

fn report_completed_session(sessions: &Sessions, now: u32) {
    let today = crate::time::day_start_epoch(now);
    let yesterday = crate::time::previous_day_start_epoch(now);

    crate::SYNC_SIGNAL.signal([
        DailyTotal {
            date: today,
            minutes: make_day_minutes(sessions, today),
        },
        DailyTotal {
            date: yesterday,
            minutes: make_day_minutes(sessions, yesterday),
        },
    ]);
}
