// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::measurement::*;
use crate::app::parse::traits::{FixedSize, FixedSizeVariation};
use crate::app::variations::{Group51Var1, Group51Var2, Variation};
use crate::app::QualifierCode;
use crate::outstation::database::details::event::traits::{Encoder, EventVariation};
use crate::outstation::database::details::event::write_fn::{
    write_cto, write_cto_first, write_fixed_size, write_octet_string, Continue, ToVariationCto,
};

use crate::util::BadWrite;
use scursor::{WriteCursor, WriteError};

/// The object header that events are written in
#[derive(Copy, Clone, PartialEq)]
enum EventHeader {
    /// a variation whose events don't depend on each other
    Plain(Variation),
    /// a variation whose event times are relative to the common time object before the header
    Cto(Variation, Time),
}

impl EventHeader {
    fn variation(self) -> Variation {
        match self {
            Self::Plain(variation) => variation,
            Self::Cto(variation, _) => variation,
        }
    }
}

/// the header an event starts
struct StartHeader<'a, T>(&'a T);

impl<T> Encoder<T> for StartHeader<'_, T> {
    type Output = Result<EventHeader, WriteError>;

    fn fixed<V>(self) -> Self::Output
    where
        V: FixedSizeVariation,
        T: ToVariation<V>,
    {
        Ok(EventHeader::Plain(V::VARIATION))
    }

    fn cto<V>(self) -> Self::Output
    where
        V: FixedSizeVariation,
        T: ToVariationCto<V>,
    {
        Ok(EventHeader::Cto(V::VARIATION, self.0.get_time().into()))
    }

    fn octet_string(self) -> Self::Output
    where
        T: AsRef<[u8]>,
    {
        let len = u8::try_from(self.0.as_ref().len()).map_err(|_| WriteError::NumericOverflow)?;
        Ok(EventHeader::Plain(Variation::Group111(len)))
    }
}

/// write an event as the first in the header it started
struct WriteFirst<'a, 'b, T> {
    cursor: &'a mut WriteCursor<'b>,
    event: &'a T,
    index: u16,
}

impl<T> Encoder<T> for WriteFirst<'_, '_, T> {
    type Output = Result<(), WriteError>;

    fn fixed<V>(self) -> Self::Output
    where
        V: FixedSizeVariation,
        T: ToVariation<V>,
    {
        write_fixed_size::<V, T>(self.cursor, self.event, self.index)
    }

    fn cto<V>(self) -> Self::Output
    where
        V: FixedSizeVariation,
        T: ToVariationCto<V>,
    {
        write_cto_first::<V, T>(self.cursor, self.event, self.index)
    }

    fn octet_string(self) -> Self::Output
    where
        T: AsRef<[u8]>,
    {
        write_octet_string(self.cursor, self.event.as_ref(), self.index)
    }
}

/// write an event after others in a header, if the header can hold it
struct Append<'a, 'b, T> {
    cursor: &'a mut WriteCursor<'b>,
    event: &'a T,
    index: u16,
    header: EventHeader,
}

impl<T> Encoder<T> for Append<'_, '_, T> {
    type Output = Result<Continue, WriteError>;

    fn fixed<V>(self) -> Self::Output
    where
        V: FixedSizeVariation,
        T: ToVariation<V>,
    {
        if self.header != EventHeader::Plain(V::VARIATION) {
            return Ok(Continue::NewHeader);
        }
        write_fixed_size::<V, T>(self.cursor, self.event, self.index)?;
        Ok(Continue::Ok)
    }

    fn cto<V>(self) -> Self::Output
    where
        V: FixedSizeVariation,
        T: ToVariationCto<V>,
    {
        let EventHeader::Cto(variation, cto) = self.header else {
            return Ok(Continue::NewHeader);
        };
        if variation != V::VARIATION {
            return Ok(Continue::NewHeader);
        }
        write_cto::<V, T>(self.cursor, self.event, self.index, cto)
    }

    fn octet_string(self) -> Self::Output
    where
        T: AsRef<[u8]>,
    {
        let bytes = self.event.as_ref();
        let same_length = u8::try_from(bytes.len())
            .is_ok_and(|len| self.header == EventHeader::Plain(Variation::Group111(len)));
        if !same_length {
            return Ok(Continue::NewHeader);
        }
        write_octet_string(self.cursor, bytes, self.index)?;
        Ok(Continue::Ok)
    }
}

#[derive(Copy, Clone)]
pub(crate) struct HeaderState {
    /// number of events that have been written
    count: u16,
    /// position where the final 2-byte count must be written
    count_position: usize,
}

impl HeaderState {
    fn new(count_position: usize) -> Self {
        Self {
            count: 1,
            count_position,
        }
    }

    /// the state after one more event, or `None` if the count is full
    fn increment(self) -> Option<Self> {
        Some(Self {
            count: self.count.checked_add(1)?,
            ..self
        })
    }
}

#[derive(Copy, Clone)]
enum State {
    Start,
    InProgress(HeaderState, EventHeader),
    Full,
}

pub(crate) struct EventWriter {
    state: State,
}

impl EventWriter {
    pub(crate) fn new() -> Self {
        Self {
            state: State::Start,
        }
    }

    pub(crate) fn write<T, V>(
        &mut self,
        cursor: &mut WriteCursor,
        event: &T,
        index: u16,
        variation: V,
    ) -> Result<(), BadWrite>
    where
        V: EventVariation<T>,
    {
        // we don't have to worry about setting state to FULL on errors since this traps all errors
        let result = self.try_write(cursor, event, index, variation.for_event(event));
        if result.is_err() {
            self.state = State::Full;
        }
        result
    }

    fn try_write<T, V>(
        &mut self,
        cursor: &mut WriteCursor,
        event: &T,
        index: u16,
        variation: V,
    ) -> Result<(), BadWrite>
    where
        V: EventVariation<T>,
    {
        let (state, header) = match self.state {
            State::Full => return Err(BadWrite),
            State::Start => return self.start_new_header(cursor, event, index, variation),
            State::InProgress(state, header) => (state, header),
        };

        // a header whose count is full needs a new header
        let Some(new_state) = state.increment() else {
            return self.start_new_header(cursor, event, index, variation);
        };

        let result = cursor.transaction(|cursor| {
            variation.encode(Append {
                cursor,
                event,
                index,
                header,
            })
        })?;

        match result {
            Continue::Ok => {
                // write the new count at the count position
                cursor.at_pos(state.count_position, |cur| {
                    cur.write_u16_le(new_state.count)
                })?;
                self.state = State::InProgress(new_state, header);
                Ok(())
            }
            Continue::NewHeader => self.start_new_header(cursor, event, index, variation),
        }
    }

    fn write_event_header(
        cursor: &mut WriteCursor,
        variation: Variation,
    ) -> Result<usize, WriteError> {
        variation.write(cursor)?;
        cursor.write_u8(QualifierCode::CountAndPrefix16.as_u8())?;
        let count_pos = cursor.position();
        cursor.write_u16_le(1)?;
        Ok(count_pos)
    }

    fn write_cto_header(cursor: &mut WriteCursor, cto: Time) -> Result<(), WriteError> {
        let time = cto.timestamp();
        if cto.is_synchronized() {
            Self::write_cto_object(cursor, &Group51Var1 { time })
        } else {
            Self::write_cto_object(cursor, &Group51Var2 { time })
        }
    }

    fn write_cto_object<C>(cursor: &mut WriteCursor, cto: &C) -> Result<(), WriteError>
    where
        C: FixedSize + FixedSizeVariation,
    {
        C::VARIATION.write(cursor)?;
        cursor.write_u8(QualifierCode::Count8.as_u8())?;
        cursor.write_u8(1)?;
        cto.write(cursor)?;
        Ok(())
    }

    fn start_new_header<T, V>(
        &mut self,
        cursor: &mut WriteCursor,
        event: &T,
        index: u16,
        variation: V,
    ) -> Result<(), BadWrite>
    where
        V: EventVariation<T>,
    {
        let header = variation.encode(StartHeader(event))?;

        let count_pos: usize = cursor.transaction(|cursor| {
            if let EventHeader::Cto(_, cto) = header {
                Self::write_cto_header(cursor, cto)?;
            }
            let count_pos = Self::write_event_header(cursor, header.variation())?;
            variation.encode(WriteFirst {
                cursor,
                event,
                index,
            })?;
            Ok(count_pos)
        })?;

        self.state = State::InProgress(HeaderState::new(count_pos), header);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::app::measurement::Flags;
    use crate::outstation::database::variations::*;

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
                6,
                EventBinaryInputVariation::Group2Var1,
            )
            .unwrap();
        assert_eq!(
            cursor.written(),
            &[
                // g2v3 (count == 1) (index == 6) (flags == 0x81)
                2, 1, 0x28, 0x01, 0x00, 0x06, 0x00, 0x81
            ]
        );

        writer
            .write(
                &mut cursor,
                &value,
                7,
                EventBinaryInputVariation::Group2Var1,
            )
            .unwrap();
        assert_eq!(
            cursor.written(),
            &[
                // same as before but with count == 2
                2, 1, 0x28, 0x02, 0x00, 0x06, 0x00, 0x81, // next index and value
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
            2, 1, 0x28, 0x01, 0x00, 0x06, 0x00, 0x81,
        ];

        {
            let value = BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0));
            writer
                .write(
                    &mut cursor,
                    &value,
                    6,
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
                    7,
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
                6,
                EventBinaryInputVariation::Group2Var1,
            )
            .unwrap();

        assert_eq!(
            cursor.written(),
            &[
                // g1v2 count == 1
                2, 1, 0x28, 0x01, 0x00, 0x06, 0x00, 0x81
            ]
        );

        writer
            .write(
                &mut cursor,
                &analog,
                7,
                EventAnalogInputVariation::Group32Var1,
            )
            .unwrap();

        assert_eq!(
            cursor.written(),
            &[
                // same as before
                2, 1, 0x28, 0x01, 0x00, 0x06, 0x00, 0x81,
                // g32v1 (count == 1) (index == 7) (flags == 0x01) (value == 27)
                32, 1, 0x28, 0x01, 0x00, 7, 0, 0x01, 27, 0, 0, 0
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
                    6,
                    EventBinaryInputVariation::Group2Var3,
                )
                .unwrap();
            assert_eq!(
                cursor.written(),
                &[
                    // synchronized CTO header
                    51, 1, 0x07, 0x01, 1, 0, 0, 0, 0, 0,
                    // g2v3 (count of 1) with relative timestamp of zero
                    2, 3, 0x28, 0x01, 0x00, 6, 00, 0x81, 0, 0,
                ]
            );
        }

        {
            let value = BinaryInput::new(false, Flags::ONLINE, Time::synchronized(28));
            writer
                .write(
                    &mut cursor,
                    &value,
                    7,
                    EventBinaryInputVariation::Group2Var3,
                )
                .unwrap();

            assert_eq!(
                cursor.written(),
                &[
                    // same CTO
                    51, 1, 0x07, 0x01, 1, 0, 0, 0, 0, 0, // count updated to 2
                    2, 3, 0x28, 0x02, 0x00, 6, 0, 0x81, 0, 0,
                    // new index, value, and relative timestamp of 27
                    7, 0, 0x01, 27, 0
                ]
            );
        }
    }

    fn write_g2v2(writer: &mut EventWriter, cursor: &mut WriteCursor, index: u16, time: Time) {
        let value = BinaryInput::new(true, Flags::ONLINE, time);
        writer
            .write(cursor, &value, index, EventBinaryInputVariation::Group2Var2)
            .unwrap();
    }

    #[test]
    fn writes_synchronized_events_selected_as_g2v2_in_g2v2() {
        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(&mut buffer);
        let mut writer = EventWriter::new();

        write_g2v2(&mut writer, &mut cursor, 6, Time::synchronized(1));
        write_g2v2(&mut writer, &mut cursor, 7, Time::synchronized(2));

        assert_eq!(
            cursor.written(),
            &[
                // g2v2 (count of 2)
                2, 2, 0x28, 0x02, 0x00, // index 6 at 1
                6, 0, 0x81, 1, 0, 0, 0, 0, 0, // index 7 at 2
                7, 0, 0x81, 2, 0, 0, 0, 0, 0,
            ]
        );
    }

    #[test]
    fn writes_unsynchronized_events_selected_as_g2v2_in_g2v3_after_an_unsynchronized_cto() {
        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(&mut buffer);
        let mut writer = EventWriter::new();

        write_g2v2(&mut writer, &mut cursor, 6, Time::unsynchronized(1));
        write_g2v2(&mut writer, &mut cursor, 7, Time::unsynchronized(3));

        assert_eq!(
            cursor.written(),
            &[
                // unsynchronized CTO at 1
                51, 2, 0x07, 0x01, 1, 0, 0, 0, 0, 0, // g2v3 (count of 2)
                2, 3, 0x28, 0x02, 0x00, // index 6 at +0
                6, 0, 0x81, 0, 0, // index 7 at +2
                7, 0, 0x81, 2, 0,
            ]
        );
    }

    #[test]
    fn writes_mixed_events_selected_as_g2v2_in_order_of_occurrence() {
        let mut buffer = [0u8; 128];
        let mut cursor = WriteCursor::new(&mut buffer);
        let mut writer = EventWriter::new();

        write_g2v2(&mut writer, &mut cursor, 1, Time::synchronized(1));
        write_g2v2(&mut writer, &mut cursor, 2, Time::unsynchronized(2));
        write_g2v2(&mut writer, &mut cursor, 3, Time::unsynchronized(65538));
        write_g2v2(&mut writer, &mut cursor, 4, Time::synchronized(65539));

        assert_eq!(
            cursor.written(),
            &[
                // g2v2 (count of 1), index 1 at 1
                2, 2, 0x28, 0x01, 0x00, 1, 0, 0x81, 1, 0, 0, 0, 0, 0,
                // unsynchronized CTO at 2, g2v3 (count of 1), index 2 at +0
                51, 2, 0x07, 0x01, 2, 0, 0, 0, 0, 0, 2, 3, 0x28, 0x01, 0x00, 2, 0, 0x81, 0, 0,
                // 65536 after the CTO: unsynchronized CTO at 65538, g2v3 (count of 1), index 3 at +0
                51, 2, 0x07, 0x01, 2, 0, 1, 0, 0, 0, 2, 3, 0x28, 0x01, 0x00, 3, 0, 0x81, 0, 0,
                // g2v2 (count of 1), index 4 at 65539
                2, 2, 0x28, 0x01, 0x00, 4, 0, 0x81, 3, 0, 1, 0, 0, 0,
            ]
        );
    }

    fn write_binary(
        writer: &mut EventWriter,
        cursor: &mut WriteCursor,
        index: u16,
        time: Option<Time>,
        variation: EventBinaryInputVariation,
    ) {
        let value = BinaryInput {
            value: true,
            flags: Flags::ONLINE,
            time,
        };
        writer.write(cursor, &value, index, variation).unwrap();
    }

    #[test]
    fn writes_events_without_time_selected_as_g2v2_in_g2v1() {
        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(&mut buffer);
        let mut writer = EventWriter::new();

        for (index, time) in [
            (1, Some(Time::synchronized(1))),
            (2, None),
            (3, Some(Time::unsynchronized(2))),
        ] {
            write_binary(
                &mut writer,
                &mut cursor,
                index,
                time,
                EventBinaryInputVariation::Group2Var2,
            );
        }

        assert_eq!(
            cursor.written(),
            &[
                // g2v2 (count of 1), index 1 at 1
                2, 2, 0x28, 0x01, 0x00, 1, 0, 0x81, 1, 0, 0, 0, 0, 0,
                // g2v1 (count of 1), index 2
                2, 1, 0x28, 0x01, 0x00, 2, 0, 0x81,
                // unsynchronized CTO at 2, g2v3 (count of 1), index 3 at +0
                51, 2, 0x07, 0x01, 2, 0, 0, 0, 0, 0, 2, 3, 0x28, 0x01, 0x00, 3, 0, 0x81, 0, 0,
            ]
        );
    }

    #[test]
    fn writes_events_without_time_selected_as_g2v3_in_g2v1() {
        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(&mut buffer);
        let mut writer = EventWriter::new();

        for (index, time) in [(1, None), (2, Some(Time::synchronized(5))), (3, None)] {
            write_binary(
                &mut writer,
                &mut cursor,
                index,
                time,
                EventBinaryInputVariation::Group2Var3,
            );
        }

        assert_eq!(
            cursor.written(),
            &[
                // g2v1 (count of 1), index 1
                2, 1, 0x28, 0x01, 0x00, 1, 0, 0x81,
                // synchronized CTO at 5, g2v3 (count of 1), index 2 at +0
                51, 1, 0x07, 0x01, 5, 0, 0, 0, 0, 0, 2, 3, 0x28, 0x01, 0x00, 2, 0, 0x81, 0, 0,
                // g2v1 (count of 1), index 3
                2, 1, 0x28, 0x01, 0x00, 3, 0, 0x81,
            ]
        );
    }

    #[test]
    fn writes_double_bit_events_without_time_selected_with_time_in_g4v1() {
        for variation in [
            EventDoubleBitBinaryInputVariation::Group4Var2,
            EventDoubleBitBinaryInputVariation::Group4Var3,
        ] {
            let mut buffer = [0u8; 64];
            let mut cursor = WriteCursor::new(&mut buffer);
            let mut writer = EventWriter::new();

            let value = DoubleBitBinaryInput {
                value: DoubleBit::DeterminedOn,
                flags: Flags::ONLINE,
                time: None,
            };
            writer.write(&mut cursor, &value, 6, variation).unwrap();

            // g4v1 (count of 1), index 6
            assert_eq!(cursor.written(), &[4, 1, 0x28, 0x01, 0x00, 6, 0, 0x81]);
        }
    }

    #[test]
    fn writes_mixed_events_selected_as_g4v2_in_order_of_occurrence() {
        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(&mut buffer);
        let mut writer = EventWriter::new();

        for (index, time) in [
            (1, Time::unsynchronized(1)),
            (2, Time::synchronized(2)),
            (3, Time::synchronized(3)),
        ] {
            let value = DoubleBitBinaryInput::new(DoubleBit::DeterminedOn, Flags::ONLINE, time);
            writer
                .write(
                    &mut cursor,
                    &value,
                    index,
                    EventDoubleBitBinaryInputVariation::Group4Var2,
                )
                .unwrap();
        }

        assert_eq!(
            cursor.written(),
            &[
                // unsynchronized CTO at 1, g4v3 (count of 1), index 1 at +0
                51, 2, 0x07, 0x01, 1, 0, 0, 0, 0, 0, 4, 3, 0x28, 0x01, 0x00, 1, 0, 0x81, 0, 0,
                // g4v2 (count of 2), index 2 at 2, index 3 at 3
                4, 2, 0x28, 0x02, 0x00, 2, 0, 0x81, 2, 0, 0, 0, 0, 0, 3, 0, 0x81, 3, 0, 0, 0, 0, 0,
            ]
        );
    }
}
