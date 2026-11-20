use core::cell::RefCell;

use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use habits_stepper::display::{self, Display, DisplayEvent};
use heapless::Vec;

pub struct DisplayEvents {
    events: Mutex<CriticalSectionRawMutex, RefCell<Vec<DisplayEvent, 16>>>,
    changed: Signal<CriticalSectionRawMutex, ()>,
}

impl DisplayEvents {
    pub const fn new() -> Self {
        Self {
            events: Mutex::new(RefCell::new(Vec::new())),
            changed: Signal::new(),
        }
    }

    pub fn record(&self, event: DisplayEvent) {
        self.events.lock(|events| {
            events
                .borrow_mut()
                .push(event)
                .expect("display event capacity exceeded");
        });
        self.changed.signal(());
    }

    pub async fn wait_for_change(&self) {
        self.changed.wait().await;
    }

    pub fn snapshot(&self) -> Vec<DisplayEvent, 16> {
        self.events.lock(|events| events.borrow().clone())
    }

    pub fn take_matching(&self, matches: impl Fn(&DisplayEvent) -> bool) -> Option<DisplayEvent> {
        self.events.lock(|events| {
            let mut events = events.borrow_mut();
            let position = events.iter().position(matches)?;
            Some(events.remove(position))
        })
    }

    pub fn clear(&self) {
        self.events.lock(|events| events.borrow_mut().clear());
    }
}

struct SimulatedDisplay<'display> {
    device: display::Sh1122Device<'display>,
    events: &'static DisplayEvents,
}

impl Display for SimulatedDisplay<'_> {
    fn show(&mut self, event: &DisplayEvent) -> Result<(), display::DisplayError> {
        self.device.show(event)?;
        self.events.record(event.clone());
        Ok(())
    }
}

#[embassy_executor::task]
pub async fn run_display_task(dependency: (display::DisplayHardware, &'static DisplayEvents)) {
    let (hardware, events) = dependency;
    let device = display::initialize_display(hardware).await;
    display::run_display(&mut SimulatedDisplay { device, events }).await;
}
