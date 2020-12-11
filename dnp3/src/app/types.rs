use std::time::{Duration, SystemTime};

use crate::app::enums::{OpType, QualifierCode, TripCloseCode};
use crate::app::variations::Variation;
use crate::util::cursor::{WriteCursor, WriteError};
use chrono::{DateTime, SecondsFormat, TimeZone, Utc};
use std::convert::TryFrom;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LinkStatusResult {
    /// The other device responded with a valid `LINK_STATUS`
    Success,
    /// There was activity on the link, but it wasn't a `LINK_STATUS`
    ///
    /// The link is still alive, but the behaviour of the other device
    /// is unexpected.
    UnexpectedResponse,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Timestamp {
    value: u64,
}

impl Timestamp {
    pub const MAX_VALUE: u64 = 0x0000_FFFF_FFFF_FFFF;
    pub const OUT_OF_RANGE: &'static str = "<out of range>";

    pub fn new(value: u64) -> Self {
        Self {
            value: value & Self::MAX_VALUE,
        }
    }

    pub fn min() -> Self {
        Self::new(0)
    }

    pub fn max() -> Self {
        Self::new(Self::MAX_VALUE)
    }

    pub fn try_from_system_time(system_time: SystemTime) -> Option<Timestamp> {
        Some(Timestamp::new(
            u64::try_from(
                system_time
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .ok()?
                    .as_millis(),
            )
            .ok()?,
        ))
    }

    pub fn to_datetime_utc(self) -> Option<DateTime<Utc>> {
        Utc.timestamp_millis_opt(self.value as i64).single()
    }

    pub fn raw_value(&self) -> u64 {
        self.value
    }

    pub(crate) fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u48_le(self.value)
    }

    pub(crate) fn checked_add(self, x: Duration) -> Option<Timestamp> {
        // safe from overflow since self.value cannot possibly be larger than MAX
        let max_add = Self::MAX_VALUE - self.value;
        let millis = x.as_millis();
        if millis > max_add as u128 {
            return None;
        }
        Some(Timestamp::new(self.value + millis as u64))
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.to_datetime_utc() {
            Some(x) => write!(f, "{}", x.to_rfc3339_opts(SecondsFormat::Millis, true)),
            None => f.write_str(Timestamp::OUT_OF_RANGE),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DoubleBit {
    Intermediate,
    DeterminedOff,
    DeterminedOn,
    Indeterminate,
}

pub(crate) struct BitPair {
    pub(crate) high: bool,
    pub(crate) low: bool,
}

impl BitPair {
    pub(crate) fn new(high: bool, low: bool) -> Self {
        Self { high, low }
    }
}

impl DoubleBit {
    pub(crate) fn from(high: bool, low: bool) -> Self {
        match (high, low) {
            (false, false) => DoubleBit::Intermediate,
            (false, true) => DoubleBit::DeterminedOff,
            (true, false) => DoubleBit::DeterminedOn,
            (true, true) => DoubleBit::Indeterminate,
        }
    }

    pub(crate) fn to_bit_pair(&self) -> BitPair {
        match self {
            DoubleBit::Intermediate => BitPair::new(false, false),
            DoubleBit::DeterminedOff => BitPair::new(false, true),
            DoubleBit::DeterminedOn => BitPair::new(true, false),
            DoubleBit::Indeterminate => BitPair::new(true, true),
        }
    }

    pub(crate) fn to_byte(&self) -> u8 {
        match self {
            DoubleBit::Intermediate => 0b00,
            DoubleBit::DeterminedOff => 0b01,
            DoubleBit::DeterminedOn => 0b10,
            DoubleBit::Indeterminate => 0b11,
        }
    }
}

impl std::fmt::Display for DoubleBit {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ControlCode {
    /// This field is used in conjunction with the `op_type` field to specify a control operation
    pub tcc: TripCloseCode,
    /// Support for this field is optional. When the clear bit is set, the device shall remove pending control commands for that
    /// index and stop any control operation that is in progress for that index. The indexed point shall go to the state that it
    /// would have if the command were allowed to complete normally.
    pub clear: bool,
    /// This field is obsolete. Masters shall always set this bit to 0. Outstations that receive a
    /// g12v1 object with this bit set shall reply with `CommandStatus::NotSupported`
    pub queue: bool,
    /// This field is used in conjunction with the `tcc` field to specify a control operation
    pub op_type: OpType,
}

impl ControlCode {
    const TCC_MASK: u8 = 0b1100_0000;
    const CR_MASK: u8 = 0b0010_0000;
    const QU_MASK: u8 = 0b0001_0000;
    const OP_MASK: u8 = 0b0000_1111;

    pub fn new(tcc: TripCloseCode, op_type: OpType, clear: bool) -> Self {
        Self {
            tcc,
            clear,
            queue: false,
            op_type,
        }
    }

    pub fn from_op_type(value: OpType) -> Self {
        Self::new(TripCloseCode::Nul, value, false)
    }

    pub fn from_tcc_and_op_type(tcc: TripCloseCode, op_type: OpType) -> Self {
        Self::new(tcc, op_type, false)
    }

    pub fn from(x: u8) -> Self {
        Self {
            tcc: TripCloseCode::from((x & Self::TCC_MASK) >> 6),
            clear: x & Self::CR_MASK != 0,
            queue: x & Self::QU_MASK != 0,
            op_type: OpType::from(x & Self::OP_MASK),
        }
    }
    pub fn as_u8(self) -> u8 {
        let mut x = 0;
        x |= self.tcc.as_u8() << 6;
        if self.clear {
            x |= Self::CR_MASK;
        }
        if self.queue {
            x |= Self::QU_MASK;
        }
        x |= self.op_type.as_u8();
        x
    }
}

impl std::fmt::Display for ControlCode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "tcc: {:?} clear: {} queue: {} op_type: {:?}",
            self.tcc, self.clear, self.queue, self.op_type
        )
    }
}

impl std::fmt::Display for Variation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let (g, v) = self.to_group_and_var();
        write!(f, "g{}v{}", g, v)
    }
}

impl QualifierCode {
    pub(crate) fn description(self) -> &'static str {
        match self {
            QualifierCode::AllObjects => "all objects",
            QualifierCode::Range8 => "1-byte start/stop",
            QualifierCode::Range16 => "2-byte start/stop",
            QualifierCode::Count8 => "1-byte count of objects",
            QualifierCode::Count16 => "2-byte count of objects",
            QualifierCode::CountAndPrefix8 => "1-byte count of objects",
            QualifierCode::CountAndPrefix16 => "2-byte count of objects",
            QualifierCode::FreeFormat16 => "2-byte free format",
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn conversion_from_timestamp_to_datetime_utc_cannot_overflow() {
        let timestamp = Timestamp::new(std::u64::MAX);
        timestamp.to_datetime_utc();
    }

    #[test]
    fn timestamp_display_formatting_works_as_expected() {
        assert_eq!(format!("{}", Timestamp::min()), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            format!("{}", Timestamp::max()),
            "+10889-08-02T05:31:50.655Z"
        );
    }

    fn test_control_code_round_trip(byte: u8, cc: ControlCode) {
        assert_eq!(cc.as_u8(), byte);
        assert_eq!(ControlCode::from(byte), cc)
    }

    #[test]
    fn correctly_converts_control_code_to_and_from_u8() {
        test_control_code_round_trip(
            0b10_1_1_0100,
            ControlCode {
                tcc: TripCloseCode::Trip,
                clear: true,
                queue: true,
                op_type: OpType::LatchOff,
            },
        );

        test_control_code_round_trip(
            0b10_0_1_0100,
            ControlCode {
                tcc: TripCloseCode::Trip,
                clear: false,
                queue: true,
                op_type: OpType::LatchOff,
            },
        );

        test_control_code_round_trip(
            0b10_1_0_0100,
            ControlCode {
                tcc: TripCloseCode::Trip,
                clear: true,
                queue: false,
                op_type: OpType::LatchOff,
            },
        );

        test_control_code_round_trip(
            0b11_0_0_0000,
            ControlCode {
                tcc: TripCloseCode::Reserved,
                clear: false,
                queue: false,
                op_type: OpType::Nul,
            },
        );
    }
}
