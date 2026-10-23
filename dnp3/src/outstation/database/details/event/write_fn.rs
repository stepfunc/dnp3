// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::measurement::*;
use crate::app::parse::traits::FixedSize;
use crate::app::variations::{Group2Var3, Group4Var3};

use scursor::{WriteCursor, WriteError};

/// whether an event joined the header in progress
pub(crate) enum Continue {
    Ok,
    NewHeader,
}

pub(crate) trait ToVariationCto<V>
where
    V: FixedSize,
{
    fn get_time(&self) -> Option<Time>;
    fn to_cto_variation(&self, timestamp: u16) -> V;
}

fn write_prefixed<V>(cursor: &mut WriteCursor, variation: &V, index: u16) -> Result<(), WriteError>
where
    V: FixedSize,
{
    cursor.write_u16_le(index)?;
    variation.write(cursor)?;
    Ok(())
}

/// write an event in a fixed-size variation
pub(crate) fn write_fixed_size<V, T>(
    cursor: &mut WriteCursor,
    event: &T,
    index: u16,
) -> Result<(), WriteError>
where
    T: ToVariation<V>,
    V: FixedSize,
{
    write_prefixed(cursor, &event.to_variation(), index)
}

/// write the first event of a header whose CTO is the event's time
pub(crate) fn write_cto_first<V, T>(
    cursor: &mut WriteCursor,
    event: &T,
    index: u16,
) -> Result<(), WriteError>
where
    T: ToVariationCto<V>,
    V: FixedSize,
{
    write_prefixed(cursor, &event.to_cto_variation(0), index)
}

/// write an event relative to the CTO of the header in progress, if its time can be
pub(crate) fn write_cto<V, T>(
    cursor: &mut WriteCursor,
    event: &T,
    index: u16,
    cto: Time,
) -> Result<Continue, WriteError>
where
    T: ToVariationCto<V>,
    V: FixedSize,
{
    let time: Time = event.get_time().into();

    // do they both have the same synchronization?
    if time.is_synchronized() != cto.is_synchronized() {
        return Ok(Continue::NewHeader);
    }

    // the timestamp is before the CTO, or too far after it to encode
    let Some(difference) = time
        .timestamp()
        .raw_value()
        .checked_sub(cto.timestamp().raw_value())
        .and_then(|x| u16::try_from(x).ok())
    else {
        return Ok(Continue::NewHeader);
    };

    let variation = event.to_cto_variation(difference);

    write_prefixed(cursor, &variation, index).map(|_| Continue::Ok)
}

pub(crate) fn write_octet_string(
    cursor: &mut WriteCursor,
    event: &[u8],
    index: u16,
) -> Result<(), WriteError> {
    cursor.write_u16_le(index)?;
    cursor.write_bytes(event)
}

impl ToVariationCto<Group2Var3> for BinaryInput {
    fn get_time(&self) -> Option<Time> {
        self.time
    }

    fn to_cto_variation(&self, timestamp: u16) -> Group2Var3 {
        Group2Var3 {
            flags: self.get_wire_flags(),
            time: timestamp,
        }
    }
}

impl ToVariationCto<Group4Var3> for DoubleBitBinaryInput {
    fn get_time(&self) -> Option<Time> {
        self.time
    }

    fn to_cto_variation(&self, timestamp: u16) -> Group4Var3 {
        Group4Var3 {
            flags: self.get_wire_flags(),
            time: timestamp,
        }
    }
}
