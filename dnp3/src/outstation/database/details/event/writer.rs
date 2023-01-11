use crate::app::measurement::*;
use crate::app::parse::traits::{FixedSize, FixedSizeVariation};
use crate::app::variations::{Group51Var1, Group51Var2};
use crate::app::QualifierCode;
use crate::app::Timestamp;
use crate::outstation::database::config::*;
use crate::outstation::database::details::event::traits::{EventVariation, OctetStringLength};
use crate::outstation::database::details::event::write_fn::Continue;

use crate::util::BadWrite;
use scursor::{WriteCursor, WriteError};

#[derive(Copy, Clone)]
pub(crate) struct HeaderState {
    /// number of events that have been written
    count: u16,
    /// position where the final 2-byte count must be written
    count_position: usize,
    /// the timestamp of the first value - used for CTO variations like g2v3/g4v3
    cto: Time,
}

impl HeaderState {
    fn new(count_position: usize, cto: Time) -> Self {
        Self {
            count: 1,
            count_position,
            cto,
        }
    }

    fn increment(self) -> Self {
        Self {
            count: self.count + 1,
            ..self
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) enum HeaderType {
    Binary(EventBinaryInputVariation),
    DoubleBitBinary(EventDoubleBitBinaryInputVariation),
    BinaryOutputStatus(EventBinaryOutputStatusVariation),
    Counter(EventCounterVariation),
    FrozenCounter(EventFrozenCounterVariation),
    Analog(EventAnalogInputVariation),
    AnalogOutputStatus(EventAnalogOutputStatusVariation),
    OctetString(OctetStringLength),
}

#[derive(Copy, Clone)]
enum State {
    Start,
    InProgress(HeaderState, HeaderType),
    Full,
}

pub(crate) struct EventWriter {
    state: State,
}

pub(crate) trait Writable: Sized {
    type EventVariation: PartialEq + EventVariation<Self>;

    fn get_header_variation(&self, header: &HeaderType) -> Option<Self::EventVariation>;

    fn get_time(&self) -> Option<Time>;
}

impl EventWriter {
    pub(crate) fn new() -> Self {
        Self {
            state: State::Start,
        }
    }

    pub(crate) fn write<E>(
        &mut self,
        cursor: &mut WriteCursor,
        event: &E,
        index: u16,
        variation: E::EventVariation,
    ) -> Result<(), BadWrite>
    where
        E: Writable,
    {
        // we don't have to worry about setting state to FULL on errors since this traps all errors
        let result = self.try_write(cursor, event, index, variation);
        if result.is_err() {
            self.state = State::Full;
        }
        result
    }

    fn try_write<E>(
        &mut self,
        cursor: &mut WriteCursor,
        event: &E,
        index: u16,
        variation: E::EventVariation,
    ) -> Result<(), BadWrite>
    where
        E: Writable,
    {
        match self.state {
            State::Full => Err(BadWrite),
            State::Start => self.start_new_header(cursor, event, index, variation),
            State::InProgress(state, header_type) => {
                match event.get_header_variation(&header_type) {
                    // different type of header
                    None => self.start_new_header(cursor, event, index, variation),
                    // same type of header
                    Some(current_variation) => {
                        if current_variation != variation {
                            self.start_new_header(cursor, event, index, variation)
                        } else {
                            // if we ever wrote this number of events, let's avoid overflow
                            // this will never happen based on packet/buffer sizes but this
                            // futures proofs things
                            if state.count == u16::MAX {
                                return self.start_new_header(cursor, event, index, variation);
                            }

                            let result: Continue = cursor.transaction(|c| {
                                current_variation.write(c, event, index, state.cto)
                            })?;

                            match result {
                                Continue::Ok => {
                                    let new_state = state.increment();
                                    // write the new count at the count position
                                    cursor.at_pos(state.count_position, |cur| {
                                        cur.write_u16_le(new_state.count)
                                    })?;
                                    self.state = State::InProgress(new_state, header_type);
                                    Ok(())
                                }
                                Continue::NewHeader => {
                                    self.start_new_header(cursor, event, index, variation)
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn write_event_header(
        cursor: &mut WriteCursor,
        group: u8,
        variation: u8,
    ) -> Result<usize, WriteError> {
        cursor.write_u8(group)?;
        cursor.write_u8(variation)?;
        cursor.write_u8(QualifierCode::CountAndPrefix16.as_u8())?;
        let count_pos = cursor.position();
        cursor.write_u16_le(1)?;
        Ok(count_pos)
    }

    fn write_cto_header<T>(cursor: &mut WriteCursor, cto: &T) -> Result<(), WriteError>
    where
        T: FixedSize + FixedSizeVariation,
    {
        let (g, v) = T::VARIATION.to_group_and_var();
        cursor.write_u8(g)?;
        cursor.write_u8(v)?;
        cursor.write_u8(QualifierCode::Count8.as_u8())?;
        cursor.write_u8(1)?;
        cto.write(cursor)?;
        Ok(())
    }

    fn start_new_header<E>(
        &mut self,
        cursor: &mut WriteCursor,
        event: &E,
        index: u16,
        variation: E::EventVariation,
    ) -> Result<(), BadWrite>
    where
        E: Writable,
    {
        let time = event
            .get_time()
            .unwrap_or_else(|| Time::Unsynchronized(Timestamp::new(0)));

        let write_op = |cursor: &mut WriteCursor| -> Result<usize, WriteError> {
            if variation.uses_cto() {
                if time.is_synchronized() {
                    Self::write_cto_header(
                        cursor,
                        &Group51Var1 {
                            time: time.timestamp(),
                        },
                    )?;
                } else {
                    Self::write_cto_header(
                        cursor,
                        &Group51Var2 {
                            time: time.timestamp(),
                        },
                    )?;
                }
            }

            let (group, var) = variation.get_group_var(event);
            let count_pos = Self::write_event_header(cursor, group, var)?;
            variation.write(cursor, event, index, time).map(|_| ())?;
            Ok(count_pos)
        };

        let count_pos: usize = cursor.transaction(write_op)?;

        self.state = State::InProgress(HeaderState::new(count_pos, time), variation.wrap());
        Ok(())
    }
}

impl Writable for BinaryInput {
    type EventVariation = EventBinaryInputVariation;

    fn get_header_variation(&self, header: &HeaderType) -> Option<Self::EventVariation> {
        match header {
            HeaderType::Binary(var) => Some(*var),
            _ => None,
        }
    }

    fn get_time(&self) -> Option<Time> {
        self.time
    }
}

impl Writable for DoubleBitBinaryInput {
    type EventVariation = EventDoubleBitBinaryInputVariation;

    fn get_header_variation(&self, header: &HeaderType) -> Option<Self::EventVariation> {
        match header {
            HeaderType::DoubleBitBinary(var) => Some(*var),
            _ => None,
        }
    }

    fn get_time(&self) -> Option<Time> {
        self.time
    }
}

impl Writable for BinaryOutputStatus {
    type EventVariation = EventBinaryOutputStatusVariation;

    fn get_header_variation(&self, header: &HeaderType) -> Option<Self::EventVariation> {
        match header {
            HeaderType::BinaryOutputStatus(var) => Some(*var),
            _ => None,
        }
    }

    fn get_time(&self) -> Option<Time> {
        self.time
    }
}

impl Writable for Counter {
    type EventVariation = EventCounterVariation;

    fn get_header_variation(&self, header: &HeaderType) -> Option<Self::EventVariation> {
        match header {
            HeaderType::Counter(var) => Some(*var),
            _ => None,
        }
    }

    fn get_time(&self) -> Option<Time> {
        self.time
    }
}

impl Writable for FrozenCounter {
    type EventVariation = EventFrozenCounterVariation;

    fn get_header_variation(&self, header: &HeaderType) -> Option<Self::EventVariation> {
        match header {
            HeaderType::FrozenCounter(var) => Some(*var),
            _ => None,
        }
    }

    fn get_time(&self) -> Option<Time> {
        self.time
    }
}

impl Writable for AnalogInput {
    type EventVariation = EventAnalogInputVariation;

    fn get_header_variation(&self, header: &HeaderType) -> Option<Self::EventVariation> {
        match header {
            HeaderType::Analog(var) => Some(*var),
            _ => None,
        }
    }

    fn get_time(&self) -> Option<Time> {
        self.time
    }
}

impl Writable for AnalogOutputStatus {
    type EventVariation = EventAnalogOutputStatusVariation;

    fn get_header_variation(&self, header: &HeaderType) -> Option<Self::EventVariation> {
        match header {
            HeaderType::AnalogOutputStatus(var) => Some(*var),
            _ => None,
        }
    }

    fn get_time(&self) -> Option<Time> {
        self.time
    }
}

impl Writable for Box<[u8]> {
    type EventVariation = OctetStringLength;

    fn get_header_variation(&self, header: &HeaderType) -> Option<Self::EventVariation> {
        match header {
            HeaderType::OctetString(var) => Some(*var),
            _ => None,
        }
    }

    fn get_time(&self) -> Option<Time> {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::app::measurement::Flags;

    use super::*;

    #[test]
    fn can_write_g2v1_x2() {
        let mut buffer = [0u8; 32];
        let mut cursor = WriteCursor::new(&mut buffer);
        let mut writer = EventWriter::new();

        let value = BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0));
        writer
            .write(
                &mut cursor,
                &value,
                06,
                EventBinaryInputVariation::Group2Var1,
            )
            .unwrap();
        assert_eq!(
            cursor.written(),
            &[
                // g2v3 (count == 1) (index == 6) (flags == 0x81)
                02, 01, 0x28, 0x01, 0x00, 0x06, 0x00, 0x81
            ]
        );

        writer
            .write(
                &mut cursor,
                &value,
                07,
                EventBinaryInputVariation::Group2Var1,
            )
            .unwrap();
        assert_eq!(
            cursor.written(),
            &[
                // same as before but with count == 2
                02, 01, 0x28, 0x02, 0x00, 0x06, 0x00, 0x81, // next index and value
                0x07, 0x00, 0x81
            ]
        );
    }

    #[test]
    fn rolls_back_cursor_when_out_of_space() {
        let mut buffer = [0u8; 9];
        let mut cursor = WriteCursor::new(&mut buffer);
        let mut writer = EventWriter::new();

        let expected = [
            // g2v3 (count == 1) (index == 6) (flags == 0x81)
            02u8, 01, 0x28, 0x01, 0x00, 0x06, 0x00, 0x81,
        ];

        {
            let value = BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0));
            writer
                .write(
                    &mut cursor,
                    &value,
                    06,
                    EventBinaryInputVariation::Group2Var1,
                )
                .unwrap();
            assert_eq!(cursor.written(), &expected);
        }

        {
            let value = AnalogInput::new(32.0, Flags::ONLINE, Time::synchronized(0));
            // not enough space to write analog header
            assert!(writer
                .write(
                    &mut cursor,
                    &value,
                    07,
                    EventAnalogInputVariation::Group32Var1
                )
                .is_err());
            // written data should be the same
            assert_eq!(cursor.written(), &expected);
        }
    }

    #[test]
    fn can_write_g2v1_then_g32v1() {
        let mut buffer = [0u8; 32];
        let mut cursor = WriteCursor::new(&mut buffer);
        let mut writer = EventWriter::new();

        let binary = BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0));
        let analog = AnalogInput::new(27.0, Flags::ONLINE, Time::synchronized(27));

        writer
            .write(
                &mut cursor,
                &binary,
                06,
                EventBinaryInputVariation::Group2Var1,
            )
            .unwrap();

        assert_eq!(
            cursor.written(),
            &[
                // g1v2 count == 1
                02, 01, 0x28, 0x01, 0x00, 0x06, 0x00, 0x81
            ]
        );

        writer
            .write(
                &mut cursor,
                &analog,
                07,
                EventAnalogInputVariation::Group32Var1,
            )
            .unwrap();

        assert_eq!(
            cursor.written(),
            &[
                // same as before
                02, 01, 0x28, 0x01, 0x00, 0x06, 0x00, 0x81,
                // g32v1 (count == 1) (index == 7) (flags == 0x01) (value == 27)
                32, 01, 0x28, 0x01, 0x00, 07, 00, 0x01, 27, 0, 0, 0
            ]
        );
    }

    #[test]
    fn can_write_multiple_g2v3() {
        let mut buffer = [0u8; 32];
        let mut cursor = WriteCursor::new(&mut buffer);
        let mut writer = EventWriter::new();

        {
            let value = BinaryInput::new(true, Flags::ONLINE, Time::synchronized(1));
            writer
                .write(
                    &mut cursor,
                    &value,
                    06,
                    EventBinaryInputVariation::Group2Var3,
                )
                .unwrap();
            assert_eq!(
                cursor.written(),
                &[
                    // synchronized CTO header
                    51, 01, 0x07, 0x01, 1, 0, 0, 0, 0, 0,
                    // g2v3 (count of 1) with relative timestamp of zero
                    02, 03, 0x28, 0x01, 0x00, 06, 00, 0x81, 0, 0,
                ]
            );
        }

        {
            let value = BinaryInput::new(false, Flags::ONLINE, Time::synchronized(28));
            writer
                .write(
                    &mut cursor,
                    &value,
                    07,
                    EventBinaryInputVariation::Group2Var3,
                )
                .unwrap();

            assert_eq!(
                cursor.written(),
                &[
                    // same CTO
                    51, 01, 0x07, 0x01, 1, 0, 0, 0, 0, 0, // count updated to 2
                    02, 03, 0x28, 0x02, 0x00, 06, 00, 0x81, 0, 0,
                    // new index, value, and relative timestamp of 27
                    07, 00, 0x01, 27, 0
                ]
            );
        }
    }

    #[test]
    fn switches_cto_headers_when_time_type_difference() {
        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(&mut buffer);
        let mut writer = EventWriter::new();

        {
            let value = BinaryInput::new(true, Flags::ONLINE, Time::synchronized(1));
            writer
                .write(
                    &mut cursor,
                    &value,
                    06,
                    EventBinaryInputVariation::Group2Var3,
                )
                .unwrap();
            assert_eq!(
                cursor.written(),
                &[
                    // synchronized CTO header
                    51, 01, 0x07, 0x01, 1, 0, 0, 0, 0, 0,
                    // g2v3 (count of 1) with relative timestamp of zero
                    02, 03, 0x28, 0x01, 0x00, 06, 00, 0x81, 0, 0,
                ]
            );
        }

        {
            let value = BinaryInput::new(false, Flags::ONLINE, Time::unsynchronized(2));
            writer
                .write(
                    &mut cursor,
                    &value,
                    07,
                    EventBinaryInputVariation::Group2Var3,
                )
                .unwrap();

            assert_eq!(
                cursor.written(),
                &[
                    // synchronized CTO header
                    51, 01, 0x07, 0x01, 1, 0, 0, 0, 0, 0,
                    // g2v3 (count of 1) with relative timestamp of zero
                    02, 03, 0x28, 0x01, 0x00, 06, 00, 0x81, 0, 0,
                    // unsynchronized CTO header
                    51, 02, 0x07, 0x01, 2, 0, 0, 0, 0, 0,
                    // g2v3 (count of 1) with relative timestamp of zero
                    02, 03, 0x28, 0x01, 0x00, 07, 00, 0x01, 0, 0,
                ]
            );
        }
    }

    #[test]
    fn switches_cto_headers_when_time_difference_too_big_to_encode() {
        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(&mut buffer);
        let mut writer = EventWriter::new();

        {
            let value = BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0));
            writer
                .write(
                    &mut cursor,
                    &value,
                    06,
                    EventBinaryInputVariation::Group2Var3,
                )
                .unwrap();
            assert_eq!(
                cursor.written(),
                &[
                    // synchronized CTO header
                    51, 01, 0x07, 0x01, 0, 0, 0, 0, 0, 0,
                    // g2v3 (count of 1) with relative timestamp of zero
                    02, 03, 0x28, 0x01, 0x00, 06, 00, 0x81, 0, 0,
                ]
            );
        }

        {
            let value = BinaryInput::new(true, Flags::ONLINE, Time::synchronized(65536));
            writer
                .write(
                    &mut cursor,
                    &value,
                    06,
                    EventBinaryInputVariation::Group2Var3,
                )
                .unwrap();

            assert_eq!(
                cursor.written(),
                &[
                    // synchronized CTO header
                    51, 01, 0x07, 0x01, 0, 0, 0, 0, 0, 0,
                    // g2v3 (count of 1) with relative timestamp of zero
                    02, 03, 0x28, 0x01, 0x00, 06, 00, 0x81, 0, 0,
                    // synchronized CTO header
                    51, 01, 0x07, 0x01, 0, 0, 0x01, 0, 0, 0,
                    // g2v3 (count of 1) with relative timestamp of zero
                    02, 03, 0x28, 0x01, 0x00, 06, 00, 0x81, 0, 0,
                ]
            );
        }
    }
}
