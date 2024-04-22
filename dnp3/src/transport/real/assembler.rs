use crate::link::header::FrameInfo;
use crate::transport::real::header::Header;
use crate::transport::{Fragment, FragmentAddr, FragmentInfo};
use crate::util::buffer::Buffer;

#[derive(Copy, Clone)]
enum InternalState {
    Empty,
    // last info, header, and accumulated length
    Running(FrameInfo, Header, usize),
    // buffer contains an assembled ADU
    Complete(FragmentInfo, usize),
}

impl InternalState {
    fn to_assembly_state(self) -> AssemblyState {
        match self {
            InternalState::Complete(_, _) => AssemblyState::Complete,
            _ => AssemblyState::ReadMore,
        }
    }
}

pub(crate) enum AssemblyState {
    Complete,
    ReadMore,
}

pub(crate) struct Assembler {
    state: InternalState,
    // assembled count
    frame_id: u32,
    buffer: Buffer,
}

impl Assembler {
    pub(crate) fn new(max_buffer_size: usize) -> Self {
        Self {
            state: InternalState::Empty,
            frame_id: 0,
            buffer: Buffer::new(max_buffer_size),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.state = InternalState::Empty;
    }

    pub(crate) fn peek(&self) -> Option<Fragment> {
        match self.state {
            InternalState::Complete(info, size) => {
                let data = self
                    .buffer
                    .get(size)
                    .expect("tracking size greater than buffer size");
                Some(Fragment { info, data })
            }
            _ => None,
        }
    }

    pub(crate) fn pop(&mut self) -> Option<Fragment> {
        match self.state {
            InternalState::Complete(info, size) => {
                self.state = InternalState::Empty;
                let data = self
                    .buffer
                    .get(size)
                    .expect("tracking size greater than buffer size");
                Some(Fragment { info, data })
            }
            _ => None,
        }
    }

    pub(crate) fn assemble(
        &mut self,
        info: FrameInfo,
        header: Header,
        payload: &[u8],
    ) -> AssemblyState {
        // FIR always clears the state
        if header.fir {
            if let InternalState::Running(info, _, size) = self.state {
                tracing::warn!(
                    "transport: received FIR - dropping {} assembled bytes from {}",
                    size,
                    info.source
                );
            }
            self.state = InternalState::Empty;
        }

        if info.broadcast.is_some() {
            if header.fir && header.fin {
                self.append(info, header, 0, payload);
            } else {
                tracing::warn!(
                    "ignoring broadcast frame with transport header fir: {} and fin: {}",
                    header.fir,
                    header.fin
                );
            }
            return self.state.to_assembly_state();
        }

        match self.state {
            InternalState::Complete(_, _) => {
                self.state = InternalState::Empty;
                self.append(info, header, 0, payload);
            }
            InternalState::Empty => {
                // ignore non-FIR segments if there was no previous frame
                if !header.fir {
                    tracing::warn!(
                        "transport: ignoring non-FIR segment from {} with no previous FIR",
                        info.source
                    );
                    return AssemblyState::ReadMore;
                }
                self.append(info, header, 0, payload);
            }
            InternalState::Running(previous_info, previous_header, length) => {
                if header.seq.value() != previous_header.seq.next() {
                    tracing::warn!("transport: conflicting addresses, previous segment with {:?}, but received {:?}", previous_info, info);
                    self.state = InternalState::Empty;
                    return AssemblyState::ReadMore;
                }
                if info != previous_info {
                    tracing::warn!("transport: conflicting addresses, previous segment with {:?}, but received {:?}", previous_info, info);
                    self.state = InternalState::Empty;
                    return AssemblyState::ReadMore;
                }
                self.append(info, header, length, payload);
            }
        }

        self.state.to_assembly_state()
    }

    fn append(&mut self, info: FrameInfo, header: Header, acc_length: usize, data: &[u8]) {
        let new_length = acc_length + data.len();

        let mut cursor = self.buffer.write_cursor();
        cursor
            .skip(acc_length)
            .expect("accumulated length is greater than the buffer size");
        match cursor.write_bytes(data) {
            Err(_) => {
                tracing::warn!(
                    "transport buffer overflow with {} bytes to write",
                    data.len()
                );
                self.state = InternalState::Empty;
            }
            Ok(_) => {
                if header.fin {
                    let frame_id = self.frame_id;
                    let addr = FragmentAddr {
                        link: info.source,
                        phys: info.phys_addr,
                    };
                    let info = FragmentInfo::new(frame_id, addr, info.broadcast);
                    self.frame_id = self.frame_id.wrapping_add(1);
                    self.state = InternalState::Complete(info, new_length)
                } else {
                    self.state = InternalState::Running(info, header, new_length)
                }
            }
        }
    }
}
