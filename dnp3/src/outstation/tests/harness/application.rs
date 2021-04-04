use std::sync::{Arc, Mutex};

use crate::app::Timestamp;
use crate::outstation::database::Database;
use crate::outstation::tests::harness::{Event, EventHandle};
use crate::outstation::traits::{OutstationApplication, RestartDelay};
use crate::outstation::{FreezeIndices, FreezeResult, FreezeType, WriteTimeResult};

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
            processing_delay: 0,
            restart_delay: None,
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
    fn write_absolute_time(&mut self, time: Timestamp) -> WriteTimeResult {
        self.events.push(Event::WriteAbsoluteTime(time));
        WriteTimeResult::Ok
    }

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
        delay
    }

    fn freeze_counter(
        &mut self,
        indices: FreezeIndices,
        freeze_type: FreezeType,
        _db: &mut Database,
    ) -> FreezeResult {
        self.events.push(Event::Freeze(indices, freeze_type));
        FreezeResult::Success
    }
}
