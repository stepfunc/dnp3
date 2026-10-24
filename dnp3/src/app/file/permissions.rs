// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

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

    fn from_value(x: u16) -> Self {
        Self {
            execute: x & 0b001 != 0,
            write: x & 0b010 != 0,
            read: x & 0b100 != 0,
        }
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

impl Permissions {
    fn value(self) -> u16 {
        self.world.value() | (self.group.value() << 3) | (self.owner.value() << 6)
    }

    pub(crate) fn write(&self, cursor: &mut WriteCursor) -> Result<(), scursor::WriteError> {
        cursor.write_u16_le(self.value())
    }

    pub(crate) fn read(cursor: &mut ReadCursor) -> Result<Self, scursor::ReadError> {
        let value = cursor.read_u16_le()?;
        Ok(Self {
            world: PermissionSet::from_value(value),
            group: PermissionSet::from_value(value >> 3),
            owner: PermissionSet::from_value(value >> 6),
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

    /// world `r--`, group `-w-` and owner `--x`, so no two classes can trade places
    const EACH_CLASS_DIFFERENT: Permissions = Permissions {
        world: PermissionSet {
            execute: false,
            write: false,
            read: true,
        },
        group: PermissionSet {
            execute: false,
            write: true,
            read: false,
        },
        owner: PermissionSet {
            execute: true,
            write: false,
            read: false,
        },
    };

    #[test]
    fn each_class_has_its_own_bits() {
        test_permission_writing(EACH_CLASS_DIFFERENT, &[0b0101_0100, 0x00]);
        test_permission_parsing(&[0b0101_0100, 0x00], EACH_CLASS_DIFFERENT);
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
