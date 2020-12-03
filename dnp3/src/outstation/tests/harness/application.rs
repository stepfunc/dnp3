use crate::outstation::tests::harness::{Event, EventHandle};
use crate::outstation::traits::{OutstationApplication, RestartDelay};
use std::sync::{Arc, Mutex};

pub(crate) struct MockOutstationApplication {
    events: EventHandle,
    data: Arc<Mutex<ApplicationData>>,
}

pub(crate) struct ApplicationData {
    pub(crate) processing_delay: u16,
    pub(crate) restart_delay: Option<RestartDelay>,
}

impl ApplicationData {
    fn new() -> Self {
        Self {
            processing_delay: 0xFFFF,
            restart_delay: Some(RestartDelay::Milliseconds(1234)),
        }
    }
}

impl MockOutstationApplication {
    pub(crate) fn new(
        events: EventHandle,
    ) -> (Arc<Mutex<ApplicationData>>, Box<dyn OutstationApplication>) {
        let data = Arc::new(Mutex::new(ApplicationData::new()));
        (data.clone(), Box::new(Self { events, data }))
    }
}

impl OutstationApplication for MockOutstationApplication {
    fn get_processing_delay_ms(&self) -> u16 {
        self.data.lock().unwrap().processing_delay
    }

    fn cold_restart(&mut self) -> Option<RestartDelay> {
        let delay = self.data.lock().unwrap().restart_delay;
        self.events.push(Event::ColdRestart(delay));
        delay
    }

    fn warm_restart(&mut self) -> Option<RestartDelay> {
        let delay = self.data.lock().unwrap().restart_delay;
        self.events.push(Event::WarmRestart(delay));
        Some(RestartDelay::Seconds(1))
    }
}
