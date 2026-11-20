use embassy_futures::select::select;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use habits_stepper::sessions::storage::FlashRing;
use habits_stepper::sessions::{Sessions, run_sessions};
use heapless::Deque;
use static_cell::ConstStaticCell;

static RESTART_REQUESTED: Signal<CriticalSectionRawMutex, ()> = Signal::new();
static RESTARTED: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub async fn restart_sessions() {
    RESTART_REQUESTED.signal(());
    RESTARTED.wait().await;
}

#[embassy_executor::task]
pub async fn run_restartable_sessions_task(mut flash: FlashRing<'static>) {
    static SESSIONS: ConstStaticCell<Sessions> = ConstStaticCell::new(Sessions {
        current: None,
        history: Deque::new(),
    });
    let sessions = SESSIONS.take();
    loop {
        select(run_sessions(sessions, &mut flash), RESTART_REQUESTED.wait()).await;
        sessions.current = None;
        sessions.history.clear();
        RESTARTED.signal(());
    }
}
