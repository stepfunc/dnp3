use crate::app::parse::parser::ParsedFragment;
use crate::app::parse::DecodeLogLevel;
use crate::app::EndpointType;
use crate::entry::EndpointAddress;
use crate::link::error::LinkError;
use crate::link::header::{AnyAddress, FrameInfo};
use crate::outstation::SelfAddressSupport;
use crate::tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::transport::{Fragment, FragmentInfo};
use crate::util::buffer::Buffer;

pub(crate) struct MockWriter {
    num_writes: usize,
}

pub(crate) struct MockReader {
    num_reads: usize,
    count: usize,
    frame_id: u32,
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
        level: DecodeLogLevel,
        _: AnyAddress,
        fragment: &[u8],
    ) -> Result<(), LinkError>
    where
        W: AsyncWrite + Unpin,
    {
        if level != DecodeLogLevel::Nothing {
            let _ = ParsedFragment::parse(level.transmit(), fragment);
        }
        io.write(fragment).await?;
        self.num_writes += 1;
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
            frame_id: 0,
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
            x => {
                let info = self
                    .info
                    .expect("call set_rx_frame_info(..) before running test");
                let fragment = Fragment {
                    info: FragmentInfo::new(self.frame_id, info.source, info.broadcast),
                    data: &self.buffer.get(x).unwrap(),
                };
                Some(fragment)
            }
        }
    }

    pub(crate) async fn read_next<T>(&mut self, io: &mut T) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        self.count = 0;
        self.num_reads += 1;
        self.count = io
            .read(self.buffer.get_mut(self.buffer.len()).unwrap())
            .await?;
        self.frame_id = self.frame_id.wrapping_add(1);
        Ok(())
    }
}
