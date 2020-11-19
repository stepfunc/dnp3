use crate::app::parse::DecodeLogLevel;
use crate::entry::NormalAddress;
use crate::link::error::LinkError;
use crate::link::header::{Address, AddressPair};
use crate::transport::{Fragment, TransportType};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub(crate) struct MockWriter {
    num_writes: usize,
}

pub(crate) struct MockReader {
    num_reads: usize,
    count: usize,
    address: AddressPair,
    buffer: [u8; 2048],
}

// same signature as the real transport writer
impl MockWriter {
    pub(crate) fn new(_: TransportType, _: NormalAddress) -> Self {
        Self { num_writes: 0 }
    }

    pub(crate) fn reset(&mut self) {}

    pub(crate) fn num_writes(&self) -> usize {
        self.num_writes
    }

    // just write the fragment directly to the I/O
    pub(crate) async fn write<W>(
        &mut self,
        io: &mut W,
        _: DecodeLogLevel,
        _: Address,
        fragment: &[u8],
    ) -> Result<(), LinkError>
    where
        W: AsyncWrite + Unpin,
    {
        self.num_writes += 1;
        println!("mock tx: {:02X?}", fragment);
        io.write(fragment).await?;
        Ok(())
    }
}

impl MockReader {
    pub(crate) fn new(_: TransportType, address: NormalAddress) -> Self {
        Self {
            num_reads: 0,
            count: 0,
            address: AddressPair::new(address.wrap(), Address::from(1024)),
            buffer: [0; 2048],
        }
    }

    pub(crate) fn num_reads(&self) -> usize {
        self.num_reads
    }

    pub(crate) fn reset(&mut self) {}

    pub(crate) fn peek(&self) -> Option<Fragment> {
        match self.count {
            0 => None,
            x => Some(Fragment {
                address: self.address,
                data: &self.buffer[0..x],
            }),
        }
    }

    pub(crate) async fn read<T>(&mut self, io: &mut T) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        self.count = io.read(&mut self.buffer).await?;
        self.num_reads += 1;
        println!("mock rx: {:02X?}", &self.buffer[0..self.count]);
        Ok(())
    }
}
