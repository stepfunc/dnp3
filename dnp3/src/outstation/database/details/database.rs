use crate::app::gen::all::AllObjectsVariation;
use crate::app::header::IIN2;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails, ObjectHeader};
use crate::outstation::database::details::event::buffer::EventBuffer;
use crate::outstation::database::details::range::static_db::{
    PointConfig, StaticDatabase, Updatable,
};
use crate::outstation::database::{EventBufferConfig, ResponseInfo, UpdateOptions};
use crate::outstation::types::EventClass;
use crate::util::cursor::WriteCursor;

pub(crate) struct Database {
    static_db: StaticDatabase,
    event_buffer: EventBuffer,
}

impl Database {
    pub(crate) fn new(config: EventBufferConfig) -> Self {
        Self {
            static_db: StaticDatabase::new(),
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

    pub(crate) fn select(&mut self, headers: &HeaderCollection) -> IIN2 {
        // ensure that any previous selection is cleared
        self.static_db.reset();

        let iin = headers.iter().fold(IIN2::default(), |iin, header| {
            iin | self.select_one(&header)
        });

        if let Some(num) = self.static_db.exceeded_capacity() {
            log::warn!(
                "READ operation contained {} more ranges than the maximum configured limit of {}",
                num,
                self.static_db.selection_capacity()
            );
            iin | IIN2::PARAMETER_ERROR
        } else {
            iin
        }
    }

    pub(crate) fn select_one(&mut self, header: &ObjectHeader) -> IIN2 {
        let result: IIN2 = match &header.details {
            // classes and specific variations
            HeaderDetails::AllObjects(var) => self.select_all_objects(*var),
            // other header types are not used for READ operations at this time
            _ => IIN2::NO_FUNC_CODE_SUPPORT,
        };

        if result.get_no_func_code_support() {
            log::warn!(
                "READ operation not supported for: {} - {}",
                header.variation,
                header.details.qualifier()
            );
        }

        result
    }

    fn select_all_objects(&mut self, variation: AllObjectsVariation) -> IIN2 {
        match variation {
            AllObjectsVariation::Group60Var1 => {
                self.static_db.select_class_0();
            }
            AllObjectsVariation::Group60Var2 => {
                self.event_buffer
                    .select_by_class(EventClass::Class1.into(), None);
            }
            AllObjectsVariation::Group60Var3 => {
                self.event_buffer
                    .select_by_class(EventClass::Class2.into(), None);
            }
            AllObjectsVariation::Group60Var4 => {
                self.event_buffer
                    .select_by_class(EventClass::Class3.into(), None);
            }
            // not supported (yet)
            _ => return IIN2::NO_FUNC_CODE_SUPPORT,
        };

        IIN2::default()
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
        let unwritten_event_classes = self.event_buffer.unwritten_classes();
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
            unwritten: unwritten_event_classes,
        }
    }
}
