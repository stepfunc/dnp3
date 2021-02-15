use crate::app::types::DoubleBit;
use crate::util::bit::{bits, BitMask, Bitfield};
use std::fmt::Formatter;
use std::ops::{BitOr, BitOrAssign};

/// Flags as defined in the specification where each bit has a type-specific meaning
///
/// Not every bit is used for every type (Binary, Analog, etc). Users
/// should refer to the standard to determine what flag values
/// correspond to each type.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Flags {
    /// underlying bitmask
    pub value: u8,
}

impl Flags {
    /// object value is 'good' / 'valid' / 'nominal'
    pub const ONLINE: Flags = Flags::new(bits::BIT_0.value);
    /// object value has not been updated since device restart
    pub const RESTART: Flags = Flags::new(bits::BIT_1.value);
    /// object value represents the last value available before a communication failure occurred
    pub const COMM_LOST: Flags = Flags::new(bits::BIT_2.value);
    /// object value is overridden in a downstream reporting device
    pub const REMOTE_FORCED: Flags = Flags::new(bits::BIT_3.value);
    /// object value is overridden by the device reporting this flag
    pub const LOCAL_FORCED: Flags = Flags::new(bits::BIT_4.value);
    /// object value is changing state rapidly (device dependent meaning)
    pub const CHATTER_FILTER: Flags = Flags::new(bits::BIT_5.value);
    /// object value exceeds the measurement range of the reported variation
    pub const OVER_RANGE: Flags = Flags::new(bits::BIT_5.value);
    /// reported counter value cannot be compared against a prior value to obtain the correct count difference
    pub const DISCONTINUITY: Flags = Flags::new(bits::BIT_6.value);
    /// object value might not have the expected level of accuracy
    pub const REFERENCE_ERR: Flags = Flags::new(bits::BIT_6.value);

    /// create a `Flags` struct from a `u8` bitmask
    pub const fn new(value: u8) -> Self {
        Self { value }
    }

    /// true if all of the flags in 'other' are set in this Flags
    pub fn is_set(&self, other: Flags) -> bool {
        (self.value & other.value) == other.value
    }
}

impl BitOr<Flags> for Flags {
    type Output = Flags;

    fn bitor(self, rhs: Flags) -> Self::Output {
        Flags::new(self.value | rhs.value)
    }
}

impl BitOrAssign<Flags> for Flags {
    fn bitor_assign(&mut self, rhs: Flags) {
        self.value |= rhs.value
    }
}

// some crate only helpers
impl Flags {
    /// test a `Flags` struct to see if the `STATE` bit is set
    pub(crate) fn state(self) -> bool {
        self.value.bit_7()
    }

    /// extract the `DoubleBit` value from a flags struct
    pub(crate) fn double_bit_state(self) -> DoubleBit {
        DoubleBit::from(self.value.bit_7(), self.value.bit_6())
    }

    pub(crate) fn with_bits_set_to(&self, mask: BitMask, value: bool) -> Flags {
        if value {
            self.with_bits_set(mask)
        } else {
            self.with_bits_cleared(mask)
        }
    }

    pub(crate) fn with_bits_cleared(&self, mask: BitMask) -> Flags {
        Flags::new(self.value & !mask.value)
    }

    pub(crate) fn with_bits_set(&self, mask: BitMask) -> Flags {
        Flags::new(self.value | mask.value)
    }

    pub(crate) fn without(&self, mask: BitMask) -> Flags {
        Flags::new(self.value & !mask.value)
    }
}

struct FlagFormatter {
    prev: bool,
}

impl FlagFormatter {
    fn new() -> Self {
        Self { prev: false }
    }

    fn push(&mut self, is_set: bool, text: &'static str, f: &mut Formatter) -> std::fmt::Result {
        if is_set {
            if self.prev {
                f.write_str(", ")?;
            }
            self.prev = true;
            f.write_str(text)?;
        }
        Ok(())
    }

    fn begin(flags: Flags, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "0x{:02X} [", flags.value)
    }

    fn end(f: &mut Formatter) -> std::fmt::Result {
        f.write_str("]")
    }

    fn format_binary_flags_0_to_4(&mut self, flags: Flags, f: &mut Formatter) -> std::fmt::Result {
        self.push(flags.is_set(Flags::ONLINE), "ONLINE", f)?;
        self.push(flags.is_set(Flags::RESTART), "RESTART", f)?;
        self.push(flags.is_set(Flags::COMM_LOST), "COMM_LOST", f)?;
        self.push(flags.is_set(Flags::REMOTE_FORCED), "REMOTE_FORCED", f)?;
        self.push(flags.is_set(Flags::LOCAL_FORCED), "LOCAL_FORCED", f)?;
        Ok(())
    }

    fn format_binary_flags_0_to_5(&mut self, flags: Flags, f: &mut Formatter) -> std::fmt::Result {
        self.format_binary_flags_0_to_4(flags, f)?;
        self.push(flags.is_set(Flags::CHATTER_FILTER), "CHATTER_FILTER", f)?;
        Ok(())
    }

    fn push_debug_item<T>(
        &mut self,
        name: &'static str,
        item: T,
        f: &mut Formatter,
    ) -> std::fmt::Result
    where
        T: std::fmt::Debug,
    {
        if self.prev {
            f.write_str(", ")?;
        }
        self.prev = true;
        write!(f, "{} = {:?}", name, item)
    }
}

pub(crate) struct BinaryFlagFormatter {
    flags: Flags,
}

impl BinaryFlagFormatter {
    pub(crate) fn new(value: u8) -> Self {
        Self {
            flags: Flags::new(value),
        }
    }
}

impl std::fmt::Display for BinaryFlagFormatter {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let mut formatter = FlagFormatter::new();
        FlagFormatter::begin(self.flags, f)?;
        formatter.format_binary_flags_0_to_5(self.flags, f)?;
        formatter.push(self.flags.value.bit_6(), "RESERVED(6)", f)?;
        formatter.push(self.flags.value.bit_7(), "STATE", f)?;
        FlagFormatter::end(f)
    }
}

pub(crate) struct DoubleBitBinaryFlagFormatter {
    flags: Flags,
}

impl DoubleBitBinaryFlagFormatter {
    pub(crate) fn new(value: u8) -> Self {
        Self {
            flags: Flags::new(value),
        }
    }
}

impl std::fmt::Display for DoubleBitBinaryFlagFormatter {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let mut formatter = FlagFormatter::new();
        FlagFormatter::begin(self.flags, f)?;
        formatter.format_binary_flags_0_to_5(self.flags, f)?;
        formatter.push_debug_item("state", self.flags.double_bit_state(), f)?;
        FlagFormatter::end(f)
    }
}

pub(crate) struct BinaryOutputStatusFlagFormatter {
    flags: Flags,
}

impl BinaryOutputStatusFlagFormatter {
    pub(crate) fn new(value: u8) -> Self {
        Self {
            flags: Flags::new(value),
        }
    }
}

impl std::fmt::Display for BinaryOutputStatusFlagFormatter {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let mut formatter = FlagFormatter::new();
        FlagFormatter::begin(self.flags, f)?;
        formatter.format_binary_flags_0_to_4(self.flags, f)?;
        formatter.push(self.flags.value.bit_5(), "RESERVED(5)", f)?;
        formatter.push(self.flags.value.bit_6(), "RESERVED(6)", f)?;
        formatter.push(self.flags.value.bit_7(), "STATE", f)?;
        FlagFormatter::end(f)
    }
}

pub(crate) struct CounterFlagFormatter {
    flags: Flags,
}

impl CounterFlagFormatter {
    pub(crate) fn new(value: u8) -> Self {
        Self {
            flags: Flags::new(value),
        }
    }
}

impl std::fmt::Display for CounterFlagFormatter {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let mut formatter = FlagFormatter::new();
        FlagFormatter::begin(self.flags, f)?;
        formatter.format_binary_flags_0_to_4(self.flags, f)?;
        formatter.push(self.flags.value.bit_5(), "ROLLOVER", f)?;
        formatter.push(self.flags.value.bit_6(), "DISCONTINUITY", f)?;
        formatter.push(self.flags.value.bit_7(), "RESERVED(7)", f)?;
        FlagFormatter::end(f)
    }
}

pub(crate) struct AnalogFlagFormatter {
    flags: Flags,
}

impl AnalogFlagFormatter {
    pub(crate) fn new(value: u8) -> Self {
        Self {
            flags: Flags::new(value),
        }
    }
}

impl std::fmt::Display for AnalogFlagFormatter {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let mut formatter = FlagFormatter::new();
        FlagFormatter::begin(self.flags, f)?;
        formatter.format_binary_flags_0_to_4(self.flags, f)?;
        formatter.push(self.flags.value.bit_5(), "OVER_RANGE", f)?;
        formatter.push(self.flags.value.bit_6(), "REFERENCE_ERR", f)?;
        formatter.push(self.flags.value.bit_7(), "RESERVED(7)", f)?;
        FlagFormatter::end(f)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bit_or_works() {
        let flags = Flags::ONLINE | Flags::LOCAL_FORCED;
        assert_eq!(flags.value, 0b0001_0001);
    }

    #[test]
    fn bit_or_assign_works() {
        let mut flags = Flags::ONLINE;
        flags |= Flags::LOCAL_FORCED;
        assert_eq!(flags.value, 0b0001_0001);
    }

    #[test]
    fn formats_binary_flags() {
        assert_eq!(format!("{}", BinaryFlagFormatter::new(0)), "0x00 []");
        assert_eq!(
            format!("{}", BinaryFlagFormatter::new(0b1100_0001)),
            "0xC1 [ONLINE, RESERVED(6), STATE]"
        );
    }

    #[test]
    fn formats_double_flags() {
        assert_eq!(
            format!("{}", DoubleBitBinaryFlagFormatter::new(0)),
            "0x00 [state = Intermediate]"
        );
        assert_eq!(
            format!("{}", DoubleBitBinaryFlagFormatter::new(0b1100_0001)),
            "0xC1 [ONLINE, state = Indeterminate]"
        );
    }
}
