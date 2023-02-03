use scursor::{ReadCursor, WriteCursor};
use std::fmt::Write;

/// Defines read, write, execute permissions for particular group or user
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct PermissionSet {
    /// Permission to execute
    pub execute: bool,
    /// Permission to write
    pub write: bool,
    /// Permission to read
    pub read: bool,
}

impl std::fmt::Display for PermissionSet {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_char(if self.read { 'r' } else { '-' })?;
        f.write_char(if self.write { 'w' } else { '-' })?;
        f.write_char(if self.execute { 'x' } else { '-' })?;
        Ok(())
    }
}

impl PermissionSet {
    /// Construct a permission set that allows read, write, and execute
    pub const fn all() -> Self {
        Self {
            execute: true,
            write: true,
            read: true,
        }
    }

    /// Construct a permission set that indicates read-only access
    pub const fn read_only() -> Self {
        Self {
            execute: false,
            write: false,
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

/// Permissions for world, group, and owner
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Permissions {
    /// World permissions
    pub world: PermissionSet,
    /// Group permissions
    pub group: PermissionSet,
    /// Owner permissions
    pub owner: PermissionSet,
}

impl Permissions {
    /// Construct Permissions indicating the same permissions for all users
    pub const fn same(set: PermissionSet) -> Permissions {
        Permissions {
            world: set,
            group: set,
            owner: set,
        }
    }
}

impl std::fmt::Display for Permissions {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "world: {} group: {} owner: {}",
            self.world, self.group, self.owner
        )
    }
}

#[derive(Copy, Clone)]
struct Mask(u16);

impl Mask {
    const fn bit(bit: u8) -> Self {
        Self(1 << bit)
    }
    fn is_set(self, value: u16) -> bool {
        self.0 & value != 0
    }
}

impl Permissions {
    const WE: Mask = Mask::bit(0);
    const WW: Mask = Mask::bit(1);
    const WR: Mask = Mask::bit(2);

    const GE: Mask = Mask::bit(3);
    const GW: Mask = Mask::bit(4);
    const GR: Mask = Mask::bit(5);

    const OE: Mask = Mask::bit(6);
    const OW: Mask = Mask::bit(7);
    const OR: Mask = Mask::bit(8);

    fn value(self) -> u16 {
        self.world.value() | self.group.value() << 3 | self.owner.value() << 6
    }

    pub(crate) fn write(&self, cursor: &mut WriteCursor) -> Result<(), scursor::WriteError> {
        cursor.write_u16_le(self.value())
    }

    pub(crate) fn read(cursor: &mut ReadCursor) -> Result<Self, scursor::ReadError> {
        let bits = cursor.read_u16_le()?;
        Ok(Self {
            world: PermissionSet {
                execute: Self::WE.is_set(bits),
                write: Self::WW.is_set(bits),
                read: Self::WR.is_set(bits),
            },
            group: PermissionSet {
                execute: Self::GE.is_set(bits),
                write: Self::GW.is_set(bits),
                read: Self::GR.is_set(bits),
            },
            owner: PermissionSet {
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

    fn test_permission_parsing(bytes: &[u8], expected: Permissions) {
        let mut cursor = ReadCursor::new(bytes);
        let parsed = Permissions::read(&mut cursor).unwrap();
        assert!(cursor.is_empty());
        assert_eq!(expected, parsed);
    }

    fn test_permission_writing(value: Permissions, expected: &[u8]) {
        let mut buffer: [u8; 2] = [0; 2];
        let mut cursor = WriteCursor::new(&mut buffer);
        value.write(&mut cursor).unwrap();
        assert_eq!(cursor.remaining(), 0);
        assert_eq!(cursor.written(), expected);
    }

    #[test]
    fn writes_permission_bytes() {
        test_permission_writing(Permissions::same(PermissionSet::all()), &[0xFF, 0x01]);
        test_permission_writing(
            Permissions::same(PermissionSet::read_only()),
            &[0b00100100, 1],
        );
    }

    #[test]
    fn parses_permission_bytes() {
        test_permission_parsing(&[0xFF, 0x01], Permissions::same(PermissionSet::all()));
        test_permission_parsing(
            &[0b00100100, 1],
            Permissions::same(PermissionSet::read_only()),
        );
    }
}
