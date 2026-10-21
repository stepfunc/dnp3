use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::header::FrameInfo;
use crate::transport::{Fragment, FragmentAddr, FragmentInfo, TransportData, TransportRead};
use crate::util::buffer::Buffer;
use crate::util::phys::{PhysAddr, PhysLayer};

pub(crate) struct MockReader {
    count: usize,
    frame_id: u32,
    info: Option<FrameInfo>,
    buffer: Buffer,
}

impl MockReader {
    pub(super) fn new(buffer_size: usize) -> Self {
        Self {
            count: 0,
            frame_id: 0,
            info: None,
            buffer: Buffer::new(buffer_size),
        }
    }

    pub(crate) fn set_rx_frame_info(&mut self, info: FrameInfo) {
        self.info = Some(info)
    }

    #[expect(
        clippy::unwrap_in_result,
        reason = "a test that never sets the frame info should fail loudly"
    )]
    fn get(&self, count: usize) -> Option<Fragment<'_>> {
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
}

impl TransportRead for MockReader {
    fn peek(&self) -> Option<TransportData<'_>> {
        Some(TransportData::Fragment(self.get(self.count)?))
    }

    fn pop(&mut self) -> Option<TransportData<'_>> {
        let count = self.count;
        self.count = 0;
        Some(TransportData::Fragment(self.get(count)?))
    }

    fn reset(&mut self) {
        self.count = 0;
    }

    async fn read(&mut self, io: &mut PhysLayer, level: DecodeLevel) -> Result<(), LinkError> {
        if self.count > 0 {
            return Ok(());
        }

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
