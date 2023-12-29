pub(crate) mod get_info;
pub(crate) mod read;
pub(crate) mod write;

pub(crate) struct Filename(pub(crate) String);

#[derive(Copy, Clone, Default)]
pub(super) struct AuthKey(pub(super) u32);
#[derive(Copy, Clone, Default)]
pub(super) struct FileHandle(pub(super) u32);
#[derive(Copy, Clone, Default)]
pub(super) struct BlockNumber(pub(super) u32);

impl BlockNumber {
    const LAST_BIT: u32 = 0x80_00_00_00;
    const BOTTOM_BITS: u32 = !Self::LAST_BIT;

    pub(super) fn is_last(self) -> bool {
        (self.0 & Self::LAST_BIT) != 0
    }

    pub(super) fn bottom_bits(self) -> u32 {
        self.0 & Self::BOTTOM_BITS
    }
}

impl PartialEq for BlockNumber {
    fn eq(&self, other: &Self) -> bool {
        let b1 = self.0 & Self::BOTTOM_BITS;
        let b2 = other.0 & Self::BOTTOM_BITS;
        b1 == b2
    }
}