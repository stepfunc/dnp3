use crate::app::Iin2;
use crate::master::EventClasses;
use crate::outstation::database::details::event::buffer::EventBuffer;
use crate::outstation::database::details::range::static_db::{
    PointConfig, StaticDatabase, Updatable,
};
use crate::outstation::database::read::ReadHeader;
use crate::outstation::database::{
    ClassZeroConfig, EventBufferConfig, ResponseInfo, UpdateOptions,
};
use crate::util::cursor::WriteCursor;

pub(crate) struct Database {
    static_db: StaticDatabase,
    event_buffer: EventBuffer,
}

impl Database {
    pub(crate) fn new(
        max_read_selection: Option<u16>,
        class_zero_config: ClassZeroConfig,
        config: EventBufferConfig,
    ) -> Self {
        Self {
            static_db: StaticDatabase::new(max_read_selection, class_zero_config),
            event_buffer: EventBuffer::new(config),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.static_db.reset();
        self.event_buffer.reset();
    }

    pub(crate) fn clear_written_events(&mut self) {
        self.event_buffer.clear_written();
    }

    pub(crate) fn select_by_header(&mut self, header: ReadHeader) -> Iin2 {
        match header {
            ReadHeader::Static(header) => self.static_db.select(header),
            ReadHeader::Event(header) => {
                self.event_buffer.select_by_header(header);
                Iin2::default()
            }
        }
    }

    pub(crate) fn select_event_classes(&mut self, classes: EventClasses) -> usize {
        self.event_buffer.select_by_class(classes, None)
    }

    pub(crate) fn add<T>(&mut self, index: u16, config: PointConfig<T>) -> bool
    where
        T: Updatable,
    {
        self.static_db.add(index, config)
    }

    pub(crate) fn remove<T>(&mut self, index: u16) -> bool
    where
        T: Updatable,
    {
        self.static_db.remove::<T>(index)
    }

    pub(crate) fn update<T>(&mut self, value: &T, index: u16, options: UpdateOptions) -> bool
    where
        T: Updatable,
    {
        let event_data = self.static_db.update(value, index, options);

        // if an event should be produced, insert it into the buffer
        if let Some((variation, class)) = event_data {
            // TODO - do something with an overflow
            let _ = self.event_buffer.insert(index, class, value, variation);
        }

        true
    }

    pub(crate) fn write_response_headers(&mut self, cursor: &mut WriteCursor) -> ResponseInfo {
        // first we write events
        let result = self.event_buffer.write_events(cursor);
        let has_events = match result {
            Ok(count) => count > 0,
            Err(count) => count > 0,
        };

        let complete = if result.is_err() {
            // unable to write all the events in this response, so we can't any static data
            false
        } else {
            // write all events to we can try to write all static data
            self.static_db.write(cursor).is_ok()
        };

        ResponseInfo {
            has_events,
            complete,
        }
    }

    pub(crate) fn write_events_only(&mut self, cursor: &mut WriteCursor) -> usize {
        // doesn't matter if we wrote all of them or not
        match self.event_buffer.write_events(cursor) {
            Ok(x) => x,
            Err(x) => x,
        }
    }
}
