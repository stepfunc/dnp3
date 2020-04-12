use crate::app::parse::parser::ParseLogLevel;
use crate::link::error::LinkError;
use crate::link::header::Address;
use crate::transport::assembler::Fragment;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Copy, Clone)]
pub struct MockWriter;
#[derive(Copy, Clone)]
pub struct MockReader {
    count: usize,
    buffer: [u8; 2048],
}

// same signature as the real transport writer
impl MockWriter {
    pub fn new(_master: bool, _address: u16) -> Self {
        Self {}
    }

    pub fn mock() -> Self {
        Self {}
    }

    // just write the fragment directly to the I/O
    pub async fn write<W>(
        &mut self,
        _level: ParseLogLevel,
        io: &mut W,
        _destination: u16,
        fragment: &[u8],
    ) -> Result<(), LinkError>
    where
        W: AsyncWrite + Unpin,
    {
        io.write(fragment).await?;
        Ok(())
    }
}

impl MockReader {
    pub(crate) fn new(_master: bool, _address: u16) -> Self {
        Self::mock()
    }

    pub(crate) fn mock() -> Self {
        Self {
            count: 0,
            buffer: [0; 2048],
        }
    }

    pub(crate) fn peek(&self) -> Option<Fragment> {
        match self.count {
            0 => None,
            x => Some(Fragment {
                address: Address::new(0, 0), // TODO?
                data: &self.buffer[0..x],
            }),
        }
    }

    pub(crate) async fn read<T>(&mut self, io: &mut T) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        self.count = io.read(&mut self.buffer).await?;
        Ok(())
    }
}
