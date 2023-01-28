use scursor::{ReadCursor, WriteCursor};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Permission {
    pub(crate) execute: bool,
    pub(crate) write: bool,
    pub(crate) read: bool,
}

impl Permission {
    pub(crate) const fn all() -> Self {
        Self {
            execute: true,
            write: true,
            read: true,
        }
    }

    fn value(self) -> u16 {
        let mut x = 0;
        if self.execute {
            x |= 0b001;
        }
        if self.write {
            x |= 0b010;
        }
        if self.read {
            x |= 0b100;
        }
        x
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Permissions {
    pub(crate) world: Permission,
    pub(crate) group: Permission,
    pub(crate) owner: Permission,
}

#[derive(Copy, Clone)]
struct Mask(u16);

impl Mask {
    const fn new(bit: u8) -> Self {
        Self(1 << bit)
    }
    fn is_set(self, value: u16) -> bool {
        self.0 & value != 0
    }
}

impl Permissions {
    const WE: Mask = Mask::new(0);
    const WW: Mask = Mask::new(1);
    const WR: Mask = Mask::new(2);

    const GE: Mask = Mask::new(3);
    const GW: Mask = Mask::new(4);
    const GR: Mask = Mask::new(5);

    const OE: Mask = Mask::new(6);
    const OW: Mask = Mask::new(7);
    const OR: Mask = Mask::new(8);

    fn value(self) -> u16 {
        self.world.value() | self.group.value() << 3 | self.owner.value() << 6
    }

    pub(crate) fn write(&self, cursor: &mut WriteCursor) -> Result<(), scursor::WriteError> {
        cursor.write_u16_le(self.value())
    }

    pub(crate) fn read(cursor: &mut ReadCursor) -> Result<Self, scursor::ReadError> {
        let bits = cursor.read_u16_le()?;
        Ok(Self {
            world: Permission {
                execute: Self::WE.is_set(bits),
                write: Self::WW.is_set(bits),
                read: Self::WR.is_set(bits),
            },
            group: Permission {
                execute: Self::GE.is_set(bits),
                write: Self::GW.is_set(bits),
                read: Self::GR.is_set(bits),
            },
            owner: Permission {
                execute: Self::OE.is_set(bits),
                write: Self::OW.is_set(bits),
                read: Self::OR.is_set(bits),
            },
        })
    }
}

#[cfg(test)]

mod test {
    use super::*;

    #[test]
    fn calculates_permission_bytes() {
        let all = Permissions {
            world: Permission::all(),
            group: Permission::all(),
            owner: Permission::all(),
        };

        assert_eq!(all.value(), 0x1FF);
    }
}
