use crate::app::parse::DecodeLogLevel;
use crate::app::EndpointType;
use crate::entry::EndpointAddress;
use crate::link::error::LinkError;
use crate::link::header::{AnyAddress, FrameInfo};
use crate::outstation::SelfAddressSupport;
use crate::transport::{Fragment, FragmentInfo};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub(crate) struct MockWriter {
    num_writes: usize,
}

pub(crate) struct MockReader {
    num_reads: usize,
    count: usize,
    frame_id: u32,
    info: Option<FrameInfo>,
    buffer: [u8; 2048],
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
    pub(crate) fn master(_: EndpointAddress) -> Self {
        Self::new()
    }

    pub(crate) fn outstation(_: EndpointAddress, _: SelfAddressSupport) -> Self {
        Self::new()
    }

    fn new() -> Self {
        Self {
            num_reads: 0,
            count: 0,
            frame_id: 0,
            info: None,
            buffer: [0; 2048],
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
            x => {
                let info = self
                    .info
                    .expect("call set_rx_frame_info(..) before running test");
                let fragment = Fragment {
                    info: FragmentInfo::new(self.frame_id, info.source, info.broadcast),
                    data: &self.buffer[0..x],
                };
                Some(fragment)
            }
        }
    }

    pub(crate) async fn read_next<T>(&mut self, io: &mut T) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        self.count = io.read(&mut self.buffer).await?;
        self.num_reads += 1;
        self.frame_id = self.frame_id.wrapping_add(1);
        println!("mock rx: {:02X?}", &self.buffer[0..self.count]);
        Ok(())
    }
}
