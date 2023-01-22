use std::sync::{Arc, Mutex};

use crate::app::{MaybeAsync, Timestamp};
use crate::outstation::database::DatabaseHandle;
use crate::outstation::tests::harness::{Event, EventSender};
use crate::outstation::traits::{OutstationApplication, RequestError, RestartDelay};
use crate::outstation::{BufferState, FreezeIndices, FreezeType};

pub(crate) struct MockOutstationApplication {
    events: EventSender,
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
        events: EventSender,
    ) -> (Arc<Mutex<ApplicationData>>, Box<dyn OutstationApplication>) {
        let data = Arc::new(Mutex::new(ApplicationData::new()));
        (data.clone(), Box::new(Self { events, data }))
    }
}

impl OutstationApplication for MockOutstationApplication {
    fn get_processing_delay_ms(&self) -> u16 {
        self.data.lock().unwrap().processing_delay
    }

    fn write_absolute_time(&mut self, time: Timestamp) -> Result<(), RequestError> {
        self.events.send(Event::WriteAbsoluteTime(time));
        Ok(())
    }

    fn cold_restart(&mut self) -> Option<RestartDelay> {
        let delay = self.data.lock().unwrap().restart_delay;
        self.events.send(Event::ColdRestart(delay));
        delay
    }

    fn warm_restart(&mut self) -> Option<RestartDelay> {
        let delay = self.data.lock().unwrap().restart_delay;
        self.events.send(Event::WarmRestart(delay));
        delay
    }

    fn freeze_counter(
        &mut self,
        indices: FreezeIndices,
        freeze_type: FreezeType,
        _db: &mut DatabaseHandle,
    ) -> Result<(), RequestError> {
        self.events.send(Event::Freeze(indices, freeze_type));
        Ok(())
    }

    fn support_write_analog_dead_bands(&mut self) -> bool {
        true
    }

    fn begin_write_analog_dead_bands(&mut self) {
        self.events.send(Event::BeginWriteDeadBands);
    }

    fn write_analog_dead_band(&mut self, index: u16, dead_band: f64) {
        self.events.send(Event::WriteDeadBand(index, dead_band))
    }

    fn end_write_analog_dead_bands(&mut self) -> MaybeAsync<()> {
        self.events.send(Event::EndWriteDeadBands);
        MaybeAsync::ready(())
    }

    fn begin_confirm(&mut self) {
        self.events.send(Event::BeginConfirm);
    }

    fn event_cleared(&mut self, id: u64) {
        self.events.send(Event::Cleared(id));
    }

    fn end_confirm(&mut self, state: BufferState) -> MaybeAsync<()> {
        self.events.send(Event::EndConfirm(state));
        MaybeAsync::ready(())
    }
}
