use crate::app::parse::DecodeLogLevel;
use crate::app::EndpointType;
use crate::entry::EndpointAddress;
use crate::link::error::LinkError;
use crate::link::header::{AnyAddress, FrameInfo};
use crate::outstation::SelfAddressSupport;
use crate::tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::transport::Fragment;
use crate::util::buffer::Buffer;

pub(crate) struct MockWriter {
    num_writes: usize,
}

pub(crate) struct MockReader {
    num_reads: usize,
    count: usize,
    info: Option<FrameInfo>,
    buffer: Buffer,
}

// same signature as the real transport writer
impl MockWriter {
    pub(crate) fn new(_: EndpointType, _: EndpointAddress) -> Self {
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
        _: AnyAddress,
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
    pub(crate) fn master(_: EndpointAddress, rx_buffer_size: usize) -> Self {
        Self::new(rx_buffer_size)
    }

    pub(crate) fn outstation(
        _: EndpointAddress,
        _: SelfAddressSupport,
        rx_buffer_size: usize,
    ) -> Self {
        Self::new(rx_buffer_size)
    }

    fn new(buffer_size: usize) -> Self {
        Self {
            num_reads: 0,
            count: 0,
            info: None,
            buffer: Buffer::new(buffer_size),
        }
    }

    pub(crate) fn set_rx_frame_info(&mut self, info: FrameInfo) {
        self.info = Some(info)
    }

    pub(crate) fn num_reads(&self) -> usize {
        self.num_reads
    }

    pub(crate) fn reset(&mut self) {}

    pub(crate) fn peek(&self) -> Option<Fragment> {
        match self.count {
            0 => None,
            x => Some(Fragment {
                address: self
                    .info
                    .expect("call set_rx_frame_info(..) before running test"),
                data: &self.buffer.get(x).unwrap(),
            }),
        }
    }

    pub(crate) async fn read<T>(&mut self, io: &mut T) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        self.count = io
            .read(self.buffer.get_mut(self.buffer.len()).unwrap())
            .await?;
        self.num_reads += 1;
        println!("mock rx: {:02X?}", &self.buffer.get(self.count).unwrap());
        Ok(())
    }
}
