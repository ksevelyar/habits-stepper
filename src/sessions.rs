pub mod storage;

use core::sync::atomic::Ordering;
use defmt::info;
use embassy_time::{Duration, with_timeout};
use heapless::Deque;
use static_cell::ConstStaticCell;

use crate::display::{DISPLAY_CHANNEL, DisplayEvent};
use crate::user_input::{GpioEvent, USER_INPUT_CHANNEL};
use storage::{FlashRing, SLOT_COUNT};

const MAX_SESSIONS: usize = SLOT_COUNT as usize;
const TICK_INTERVAL: Duration = Duration::from_secs(1);

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
    fn load_history(&mut self, flash: &mut FlashRing<'static>) {
        flash.init();

        for session in flash.iterate_sessions() {
            self.history.push_back(session).unwrap();
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
            let session = self.current.take().expect("a timed-out session is present");
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

fn calculate_minutes_in_range(session: &Session, range_start: u32, range_end: u32) -> u32 {
    let overlap_start = session.start_epoch.max(range_start);
    let overlap_end = session.end_epoch.min(range_end);
    if overlap_end > overlap_start {
        (overlap_end - overlap_start) / 60
    } else {
        0
    }
}

pub fn make_day_minutes(sessions: &Sessions, day_start: u32) -> u32 {
    let day_end = crate::time::calculate_next_day_start(day_start);
    let mut minutes: u32 = 0;

    for session in sessions.history.iter().chain(sessions.current.as_ref()) {
        minutes += calculate_minutes_in_range(session, day_start, day_end);
    }

    minutes
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeekTotals {
    pub minutes: u32,
    pub steps: u32,
}

pub fn make_week_totals(sessions: &Sessions, now: u32) -> WeekTotals {
    let week = crate::time::calculate_week_range(now);
    let mut minutes: u32 = 0;
    let mut steps: u32 = 0;

    for session in sessions.history.iter().chain(sessions.current.as_ref()) {
        minutes += calculate_minutes_in_range(session, week.start, week.end);
        if session.end_epoch.min(week.end) >= session.start_epoch.max(week.start) {
            steps += session.steps;
        }
    }

    WeekTotals { minutes, steps }
}

#[embassy_executor::task]
pub async fn run_sessions_task(mut flash: FlashRing<'static>) {
    static SESSIONS: ConstStaticCell<Sessions> = ConstStaticCell::new(Sessions {
        current: None,
        history: Deque::new(),
    });
    run_sessions(SESSIONS.take(), &mut flash).await;
}

pub async fn run_sessions(sessions: &mut Sessions, flash: &mut FlashRing<'static>) {
    sessions.load_history(flash);

    loop {
        let result = with_timeout(TICK_INTERVAL, USER_INPUT_CHANNEL.receive()).await;
        let epoch = crate::time::get_epoch_seconds();

        if let Some(now) = epoch {
            let completed = sessions.tick(now, flash);

            if completed.is_some() {
                report_completed_session(sessions, now);
            }

            match result {
                Ok(event @ (GpioEvent::StepDetected | GpioEvent::HistoryReleased)) => {
                    if matches!(event, GpioEvent::StepDetected) {
                        sessions.trigger(now);
                    }
                    if crate::time::TIME_SYNCHRONIZED.load(Ordering::Acquire) {
                        let totals = make_week_totals(sessions, now);
                        DISPLAY_CHANNEL.send(DisplayEvent::Update(totals)).await;
                    }
                }
                Ok(GpioEvent::HistoryPressed) => {
                    if crate::time::TIME_SYNCHRONIZED.load(Ordering::Acquire) {
                        let current_week = crate::time::calculate_week_range(now);
                        let previous_week = current_week.start.saturating_sub(1);
                        let totals = make_week_totals(sessions, previous_week);
                        DISPLAY_CHANNEL.send(DisplayEvent::History(totals)).await;
                    }
                }
                Ok(GpioEvent::TimeSynchronized) => {
                    let totals = make_week_totals(sessions, now);
                    DISPLAY_CHANNEL.send(DisplayEvent::Update(totals)).await;
                }
                Err(_) => {}
            }
        }
    }
}

fn report_completed_session(sessions: &Sessions, now: u32) {
    let today = crate::time::calculate_day_start(now);
    let yesterday = crate::time::calculate_previous_day_start(now);

    crate::sync::SYNC_SIGNAL.signal([
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
