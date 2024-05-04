use crate::app::measurement::*;
use crate::app::parse::traits::FixedSize;
use crate::app::variations::{Group2Var3, Group4Var3};

use scursor::{WriteCursor, WriteError};

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

/// Conforms to the WriteFn<T> function type for any type can be converted to a variation that is fixed size
pub(crate) fn write_fixed_size<V, T>(
    cursor: &mut WriteCursor,
    event: &T,
    index: u16,
    _: Time,
) -> Result<Continue, WriteError>
where
    T: ToVariation<V>,
    V: FixedSize,
{
    write_prefixed(cursor, &event.to_variation(), index).map(|_| Continue::Ok)
}

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

    // the timestamp is < the CTO
    if cto.timestamp().raw_value() > time.timestamp().raw_value() {
        return Ok(Continue::NewHeader);
    }

    // this can't underflow b/c of check above
    let difference: u64 = time.timestamp().raw_value() - cto.timestamp().raw_value();

    // too big of a difference to encode
    if difference > u16::MAX.into() {
        return Ok(Continue::NewHeader);
    }

    let variation = event.to_cto_variation(difference as u16); // difference in range [0, u16::max]

    write_prefixed(cursor, &variation, index).map(|_| Continue::Ok)
}

pub(crate) fn write_octet_string(
    cursor: &mut WriteCursor,
    event: &[u8],
    index: u16,
) -> Result<Continue, WriteError> {
    cursor.write_u16_le(index)?;
    cursor.write_bytes(event)?;
    Ok(Continue::Ok)
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
