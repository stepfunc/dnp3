use crate::app::types::DoubleBit;
use crate::util::bit::{BitMask, Bitfield};
use std::fmt::Formatter;

/// Flags as defined in the specification where each bit has meaning.
///
/// Not every bit is used for every type (Binary, Analog, etc). Users
/// should refer to the standard to determine what flag values
/// correspond to each type.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Flags {
    /// underlying value
    pub value: u8,
}

impl Flags {
    pub(crate) const ONLINE: Flags = Flags::new(0x01);

    /// create a `Flags` struct from a `u8`
    pub const fn new(value: u8) -> Self {
        Self { value }
    }

    /// test a `Flags` struct to see if the `ONLINE` bit is set
    pub fn online(self) -> bool {
        self.value.bit_0()
    }

    /// test a `Flags` struct to see if the `RESTART` bit is set
    pub fn restart(self) -> bool {
        self.value.bit_1()
    }

    /// test a `Flags` struct to see if the `COMM_LOST` bit is set
    pub fn comm_lost(self) -> bool {
        self.value.bit_2()
    }

    /// test a `Flags` struct to see if the `REMOTE_FORCED` bit is set
    pub fn remote_forced(self) -> bool {
        self.value.bit_3()
    }

    /// test a `Flags` struct to see if the `LOCAL_FORCED` bit is set
    pub fn local_forced(self) -> bool {
        self.value.bit_4()
    }

    /// test a `Flags` struct to see if the `CHATTER_FILTER` bit is set
    pub fn chatter_filter(self) -> bool {
        self.value.bit_5()
    }

    /// test a `Flags` struct to see if the `ROLLOVER` bit is set
    pub fn rollover(self) -> bool {
        self.value.bit_5()
    }

    /// test a `Flags` struct to see if the `DISCONTINUITY` bit is set
    pub fn discontinuity(self) -> bool {
        self.value.bit_6()
    }

    /// test a `Flags` struct to see if the `OVER_RANGE` bit is set
    pub fn over_range(self) -> bool {
        self.value.bit_5()
    }

    /// test a `Flags` struct to see if the `REFERENCE_ERR` bit is set
    pub fn reference_err(self) -> bool {
        self.value.bit_6()
    }

    pub(crate) fn bit5(self) -> bool {
        self.value.bit_5()
    }

    pub(crate) fn bit6(self) -> bool {
        self.value.bit_6()
    }

    pub(crate) fn bit7(self) -> bool {
        self.value.bit_7()
    }

    /// test a `Flags` struct to see if the `STATE` bit is set
    pub fn state(self) -> bool {
        self.value.bit_7()
    }

    /// extract the `DoubleBit` value from a flags struct
    pub fn double_bit_state(self) -> DoubleBit {
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
}

pub(crate) mod format {
    use super::*;

    struct FlagFormatter {
        prev: bool,
    }

    impl FlagFormatter {
        fn new() -> Self {
            Self { prev: false }
        }

        fn push(
            &mut self,
            is_set: bool,
            text: &'static str,
            f: &mut Formatter,
        ) -> std::fmt::Result {
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

        fn format_binary_flags_0_to_4(
            &mut self,
            flags: Flags,
            f: &mut Formatter,
        ) -> std::fmt::Result {
            self.push(flags.online(), "ONLINE", f)?;
            self.push(flags.restart(), "RESTART", f)?;
            self.push(flags.comm_lost(), "COMM_LOST", f)?;
            self.push(flags.remote_forced(), "REMOTE_FORCED", f)?;
            self.push(flags.local_forced(), "LOCAL_FORCED", f)?;
            Ok(())
        }

        fn format_binary_flags_0_to_5(
            &mut self,
            flags: Flags,
            f: &mut Formatter,
        ) -> std::fmt::Result {
            self.format_binary_flags_0_to_4(flags, f)?;
            self.push(flags.chatter_filter(), "CHATTER_FILTER", f)?;
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
            formatter.push(self.flags.bit6(), "RESERVED(6)", f)?;
            formatter.push(self.flags.state(), "STATE", f)?;
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
            formatter.push(self.flags.bit5(), "RESERVED(5)", f)?;
            formatter.push(self.flags.bit6(), "RESERVED(6)", f)?;
            formatter.push(self.flags.state(), "STATE", f)?;
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
            formatter.push(self.flags.rollover(), "ROLLOVER", f)?;
            formatter.push(self.flags.discontinuity(), "DISCONTINUITY", f)?;
            formatter.push(self.flags.bit7(), "RESERVED(7)", f)?;
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
            formatter.push(self.flags.over_range(), "OVER_RANGE", f)?;
            formatter.push(self.flags.reference_err(), "REFERENCE_ERR", f)?;
            formatter.push(self.flags.bit7(), "RESERVED(7)", f)?;
            FlagFormatter::end(f)
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

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
}
