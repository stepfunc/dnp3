use super::*;
use scursor::{ReadCursor, WriteCursor};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum FileStatus {
    /// Requested operation was successful
    Success,
    /// Permission was denied due to improper authentication key, user name, or password
    PermissionDenied,
    /// An unsupported or unknown operation mode was requested
    InvalidMode,
    /// Requested file does not exist
    FileNotFound,
    /// Requested file is already in use
    FileLocked,
    /// File could not be opened because of limit on the number of open files
    TooManyOpen,
    /// There is no file opened with the handle in the request
    InvalidHandle,
    /// Outstation is unable to negotiate a suitable write block size
    WriteBlockSize,
    /// Communications were lost or cannot be establishes with end device where file resides
    CommLost,
    /// An abort request was unsuccessful because the outstation is unable or not programmed to abort
    CannotAbort,
    /// File handle does not reference an opened file
    NotOpened,
    /// File closed due to inactivity timeout
    HandleExpired,
    /// Too much file data was received for outstation to process
    BufferOverrun,
    /// An error occurred in the file processing that prevents any further activity with this file
    Fatal,
    /// The block number did not have the expected sequence number
    BlockSeq,
    /// Some other error not list here occurred. Optional text may provide further explanation.
    Undefined,
    /// Used to capture reserved values
    Reserved(u8),
}

impl FileStatus {
    fn new(value: u8) -> Self {
        match value {
            0 => Self::Success,
            1 => Self::PermissionDenied,
            2 => Self::InvalidMode,
            3 => Self::FileNotFound,
            4 => Self::FileLocked,
            5 => Self::TooManyOpen,
            6 => Self::InvalidHandle,
            7 => Self::WriteBlockSize,
            8 => Self::CommLost,
            9 => Self::CannotAbort,
            16 => Self::NotOpened,
            17 => Self::HandleExpired,
            18 => Self::BufferOverrun,
            19 => Self::Fatal,
            20 => Self::BlockSeq,
            255 => Self::Undefined,
            _ => Self::Reserved(value),
        }
    }

    fn to_u8(self) -> u8 {
        match self {
            FileStatus::Success => 0,
            FileStatus::PermissionDenied => 1,
            FileStatus::InvalidMode => 2,
            FileStatus::FileNotFound => 3,
            FileStatus::FileLocked => 4,
            FileStatus::TooManyOpen => 5,
            FileStatus::InvalidHandle => 6,
            FileStatus::WriteBlockSize => 7,
            FileStatus::CommLost => 8,
            FileStatus::CannotAbort => 9,
            FileStatus::NotOpened => 16,
            FileStatus::HandleExpired => 17,
            FileStatus::BufferOverrun => 18,
            FileStatus::Fatal => 19,
            FileStatus::BlockSeq => 20,
            FileStatus::Undefined => 255,
            FileStatus::Reserved(x) => x,
        }
    }
}

/// Group 70 Variation 4 - file command status
///
/// This representation is borrowed from the underlying ASDU
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Group70Var4<'a> {
    pub(crate) file_handle: u32,
    pub(crate) file_size: u32,
    pub(crate) max_block_size: u16,
    pub(crate) request_id: u16,
    pub(crate) status_code: FileStatus,
    pub(crate) text: &'a str,
}

impl<'a> Group70Var4<'a> {
    pub(crate) fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u32_le(self.file_handle)?;
        cursor.write_u32_le(self.file_size)?;
        cursor.write_u16_le(self.max_block_size)?;
        cursor.write_u16_le(self.request_id)?;
        cursor.write_u8(self.status_code.to_u8())?;
        cursor.write_bytes(self.text.as_bytes())?;
        Ok(())
    }

    pub(crate) fn read(cursor: &mut ReadCursor<'a>) -> Result<Self, ReadError> {
        let file_handle = cursor.read_u32_le()?;
        let file_size = cursor.read_u32_le()?;
        let max_block_size = cursor.read_u16_le()?;
        let request_id = cursor.read_u16_le()?;
        let status_code = FileStatus::new(cursor.read_u8()?);
        let text = std::str::from_utf8(cursor.read_all())?;

        Ok(Self {
            file_handle,
            file_size,
            max_block_size,
            request_id,
            status_code,
            text,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const OBJECT: Group70Var4 = Group70Var4 {
        file_handle: 0x01020304,
        file_size: 0xAABBCCDD,
        max_block_size: 1024,
        request_id: 42,
        status_code: FileStatus::FileNotFound,
        text: "wat",
    };

    const DATA: &[u8] = &[
        4, // file handle
        3, 2, 1, 0xDD, // file size
        0xCC, 0xBB, 0xAA, 00, // max block size
        04, 42, // request id
        00, 3,    // status code
        b'w', // text
        b'a', b't',
    ];

    #[test]
    fn writes_valid_g70v3() {
        let mut buffer = [0; 64];

        let mut cursor = WriteCursor::new(&mut buffer);
        OBJECT.write(&mut cursor).unwrap();

        assert_eq!(cursor.written(), DATA)
    }

    #[test]
    fn parses_valid_g70v3() {
        let mut cursor = ReadCursor::new(DATA);
        let obj = Group70Var4::read(&mut cursor).unwrap();

        assert_eq!(obj, OBJECT);
        assert!(cursor.is_empty());
    }
}
