use crate::entry::EndpointAddress;
use crate::link::error::LinkError;
use crate::link::header::FrameInfo;
use crate::outstation::config::Feature;
use crate::tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use crate::transport::{Fragment, FragmentInfo, TransportData};
use crate::util::buffer::Buffer;

pub(crate) struct MockReader {
    num_reads: usize,
    count: usize,
    frame_id: u32,
    info: Option<FrameInfo>,
    buffer: Buffer,
}

impl MockReader {
    pub(crate) fn master(
        _: EndpointAddress,
        rx_buffer_size: usize,
        _bubble_framing_errors: bool,
    ) -> Self {
        Self::new(rx_buffer_size)
    }

    pub(crate) fn outstation(
        _: EndpointAddress,
        _self_address: Feature,
        rx_buffer_size: usize,
        _bubble_framing_errors: bool,
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

    pub(crate) fn peek(&self) -> Option<TransportData> {
        Some(TransportData::Fragment(self.get(self.count)?))
    }

    pub(crate) fn pop(&mut self) -> Option<TransportData> {
        let count = self.count;
        self.count = 0;
        Some(TransportData::Fragment(self.get(count)?))
    }

    fn get(&self, count: usize) -> Option<Fragment> {
        if count == 0 {
            return None;
        }
        let info = self
            .info
            .expect("call set_rx_frame_info(..) before running test");
        let fragment = Fragment {
            info: FragmentInfo::new(self.frame_id, info.source, info.broadcast),
            data: &self.buffer.get(count).unwrap(),
        };
        Some(fragment)
    }

    pub(crate) async fn read<T>(&mut self, io: &mut T) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        if self.count > 0 {
            return Ok(());
        }

        self.num_reads += 1;
        self.count = io
            .read(self.buffer.get_mut(self.buffer.len()).unwrap())
            .await?;
        self.frame_id = self.frame_id.wrapping_add(1);
        Ok(())
    }
}
