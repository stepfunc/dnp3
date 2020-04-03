use crate::app::types::DoubleBit;

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
    pub fn new(value: u8) -> Self {
        Self { value }
    }

    pub fn new_online() -> Self {
        Self {
            value: masks::ONLINE,
        }
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

    pub fn state(self) -> bool {
        self.is_set(masks::STATE)
    }

    pub fn double_bit_state(self) -> DoubleBit {
        DoubleBit::from(self.value >> 6)
    }
}
