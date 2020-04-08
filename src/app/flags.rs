use crate::app::types::DoubleBit;
use std::fmt::Formatter;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Flags {
    pub value: u8,
}

pub mod masks {
    const fn bit(x: u8) -> u8 {
        1 << x
    }

    pub const ONLINE: u8 = bit(0);
    pub const RESTART: u8 = bit(1);
    pub const COMM_LOST: u8 = bit(2);
    pub const REMOTE_FORCED: u8 = bit(3);
    pub const LOCAL_FORCED: u8 = bit(4);
    pub const CHATTER_FILTER: u8 = bit(5);
    pub const ROLLOVER: u8 = bit(5);
    pub const OVER_RANGE: u8 = bit(5);
    pub const DISCONTINUITY: u8 = bit(6);
    pub const REFERENCE_ERR: u8 = bit(6);
    pub const STATE: u8 = bit(7);
}

impl Flags {
    pub const ONLINE: Flags = Flags::new(masks::ONLINE);

    pub const fn new(value: u8) -> Self {
        Self { value }
    }

    fn is_set(self, mask: u8) -> bool {
        self.value & mask != 0
    }

    pub fn online(self) -> bool {
        self.is_set(masks::ONLINE)
    }

    pub fn restart(self) -> bool {
        self.is_set(masks::RESTART)
    }

    pub fn comm_lost(self) -> bool {
        self.is_set(masks::COMM_LOST)
    }

    pub fn remote_forced(self) -> bool {
        self.is_set(masks::REMOTE_FORCED)
    }

    pub fn local_forced(self) -> bool {
        self.is_set(masks::LOCAL_FORCED)
    }

    pub fn chatter_filter(self) -> bool {
        self.is_set(masks::CHATTER_FILTER)
    }

    pub fn rollover(self) -> bool {
        self.is_set(masks::ROLLOVER)
    }

    pub fn discontinuity(self) -> bool {
        self.is_set(masks::DISCONTINUITY)
    }

    pub fn over_range(self) -> bool {
        self.is_set(masks::OVER_RANGE)
    }

    pub fn reference_err(self) -> bool {
        self.is_set(masks::REFERENCE_ERR)
    }

    pub fn bit6(self) -> bool {
        self.is_set(masks::DISCONTINUITY)
    }

    pub fn state(self) -> bool {
        self.is_set(masks::STATE)
    }

    pub fn double_bit_state(self) -> DoubleBit {
        DoubleBit::from(self.value >> 6)
    }
}

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
        write!(f, "0x{:02X} [", self.flags.value)?;
        let mut formatter = FlagFormatter::new();
        formatter.push(self.flags.online(), "ONLINE", f)?;
        formatter.push(self.flags.restart(), "RESTART", f)?;
        formatter.push(self.flags.comm_lost(), "COMM_LOST", f)?;
        formatter.push(self.flags.remote_forced(), "REMOTE_FORCED", f)?;
        formatter.push(self.flags.local_forced(), "LOCAL_FORCED", f)?;
        formatter.push(self.flags.chatter_filter(), "CHATTER_FILTER", f)?;
        formatter.push(self.flags.bit6(), "RESERVED_BIT_6", f)?;
        formatter.push(self.flags.state(), "STATE", f)?;
        f.write_str("]")?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::app::flags::BinaryFlagFormatter;

    #[test]
    fn formats_binary_flags() {
        assert_eq!(format!("{}", BinaryFlagFormatter::new(0)), "0x00 []");
        assert_eq!(
            format!("{}", BinaryFlagFormatter::new(0b1100_0001)),
            "0xC1 [ONLINE, RESERVED_BIT_6, STATE]"
        );
    }
}
