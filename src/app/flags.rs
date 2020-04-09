use crate::app::types::DoubleBit;
use crate::util::bit::BitTest;
use std::fmt::Formatter;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Flags {
    pub value: u8,
}

impl Flags {
    pub const ONLINE: Flags = Flags::new(0x01);

    pub const fn new(value: u8) -> Self {
        Self { value }
    }

    pub fn online(self) -> bool {
        self.value.bit_0()
    }

    pub fn restart(self) -> bool {
        self.value.bit_1()
    }

    pub fn comm_lost(self) -> bool {
        self.value.bit_2()
    }

    pub fn remote_forced(self) -> bool {
        self.value.bit_3()
    }

    pub fn local_forced(self) -> bool {
        self.value.bit_4()
    }

    pub fn chatter_filter(self) -> bool {
        self.value.bit_5()
    }

    pub fn rollover(self) -> bool {
        self.value.bit_5()
    }

    pub fn discontinuity(self) -> bool {
        self.value.bit_6()
    }

    pub fn over_range(self) -> bool {
        self.value.bit_5()
    }

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

    pub fn state(self) -> bool {
        self.value.bit_7()
    }

    pub fn double_bit_state(self) -> DoubleBit {
        DoubleBit::from(self.value.bit_7(), self.value.bit_6())
    }
}

pub(crate) mod format {
    use super::*;

    struct FlagFormatter {
        prev: bool,
    }

    impl FlagFormatter {
        pub fn new() -> Self {
            Self { prev: false }
        }

        pub fn push(
            &mut self,
            is_set: bool,
            text: &'static str,
            f: &mut std::fmt::Formatter,
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

        pub fn push_debug_item<T>(
            &mut self,
            name: &'static str,
            item: T,
            f: &mut std::fmt::Formatter,
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
        pub fn new(value: u8) -> Self {
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
        pub fn new(value: u8) -> Self {
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
        pub fn new(value: u8) -> Self {
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
        pub fn new(value: u8) -> Self {
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
        pub fn new(value: u8) -> Self {
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
