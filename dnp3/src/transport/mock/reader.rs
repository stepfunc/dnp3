// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::sync::{Arc, Mutex};

use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::header::FrameInfo;
use crate::transport::{Fragment, FragmentAddr, FragmentInfo, TransportData, TransportRead};
use crate::util::buffer::Buffer;
use crate::util::phys::{PhysAddr, PhysLayer};

/// The frame info the reader gives each fragment, which a test can change while a session runs
#[derive(Clone, Default)]
pub(crate) struct RxFrameInfo(Arc<Mutex<Option<FrameInfo>>>);

impl RxFrameInfo {
    pub(crate) fn set(&self, info: FrameInfo) {
        *self.0.lock().unwrap() = Some(info);
    }

    pub(crate) fn get(&self) -> Option<FrameInfo> {
        *self.0.lock().unwrap()
    }
}

pub(crate) struct MockReader {
    count: usize,
    info: RxFrameInfo,
    buffer: Buffer,
}

impl MockReader {
    pub(super) fn new(buffer_size: usize) -> Self {
        Self {
            count: 0,
            info: RxFrameInfo::default(),
            buffer: Buffer::new(buffer_size),
        }
    }

    pub(crate) fn set_rx_frame_info(&mut self, info: FrameInfo) {
        self.info.set(info)
    }

    pub(crate) fn rx_frame_info(&self) -> RxFrameInfo {
        self.info.clone()
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
            .get()
            .expect("call set_rx_frame_info(..) before running test");
        let addr = FragmentAddr {
            link: info.source,
            phys: PhysAddr::None,
        };
        let fragment = Fragment {
            info: FragmentInfo::new(addr, info.broadcast),
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
        Ok(())
    }
}
