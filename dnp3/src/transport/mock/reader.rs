use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::header::FrameInfo;
use crate::link::reader::LinkModes;
use crate::link::EndpointAddress;
use crate::outstation::Feature;
use crate::transport::{Fragment, FragmentAddr, FragmentInfo, TransportData};
use crate::util::buffer::Buffer;
use crate::util::phys::{PhysAddr, PhysLayer};

pub(crate) struct MockReader {
    num_reads: usize,
    count: usize,
    frame_id: u32,
    info: Option<FrameInfo>,
    buffer: Buffer,
}

impl MockReader {
    pub(crate) fn master(_: LinkModes, _: EndpointAddress, rx_buffer_size: usize) -> Self {
        Self::new(rx_buffer_size)
    }

    pub(crate) fn outstation(
        _: LinkModes,
        _: EndpointAddress,
        _self_address: Feature,
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

    pub(crate) fn seed_link(&mut self, _: &[u8]) -> Result<(), scursor::WriteError> {
        unimplemented!()
    }

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
        let addr = FragmentAddr {
            link: info.source,
            phys: PhysAddr::None,
        };
        let fragment = Fragment {
            info: FragmentInfo::new(self.frame_id, addr, info.broadcast),
            data: self.buffer.get(count).unwrap(),
        };
        Some(fragment)
    }

    pub(crate) async fn read(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
    ) -> Result<(), LinkError> {
        if self.count > 0 {
            return Ok(());
        }

        self.num_reads += 1;
        let (count, _) = io
            .read(
                self.buffer.get_mut(self.buffer.len()).unwrap(),
                level.physical,
            )
            .await?;
        self.count = count;
        self.frame_id = self.frame_id.wrapping_add(1);
        Ok(())
    }
}
