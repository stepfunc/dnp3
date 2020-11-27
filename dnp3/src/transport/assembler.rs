use crate::link::header::FrameInfo;
use crate::transport::header::Header;
use crate::transport::{Fragment, FragmentInfo};

#[derive(Copy, Clone)]
enum InternalState {
    Empty,
    // last info, header, and accumulated length
    Running(FrameInfo, Header, usize),
    // buffer contains an assembled ADU
    Complete(FragmentInfo, usize),
}

impl InternalState {
    fn to_assembly_state(&self) -> AssemblyState {
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
    // make this configurable and/or constant
    buffer: [u8; 2048],
    // assembled count
    count: usize,
}

impl Assembler {
    pub(crate) fn new() -> Self {
        Self {
            state: InternalState::Empty,
            buffer: [0; 2048],
            count: 0,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.state = InternalState::Empty;
    }

    pub(crate) fn discard(&mut self) {
        if let InternalState::Complete(_, _) = self.state {
            self.state = InternalState::Empty;
        }
    }

    pub(crate) fn peek(&self) -> Option<Fragment> {
        match self.state {
            InternalState::Complete(info, size) => Some(Fragment {
                info,
                data: &self.buffer[0..size],
            }),
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
                log::warn!(
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
                log::warn!(
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
                    log::warn!(
                        "transport: ignoring non-FIR segment from {} with no previous FIR",
                        info.source
                    );
                    return AssemblyState::ReadMore;
                }
                self.append(info, header, 0, payload);
            }
            InternalState::Running(previous_info, previous_header, length) => {
                if header.seq.value() != previous_header.seq.next() {
                    log::warn!("transport: conflicting addresses, previous segment with {:?}, but received {:?}", previous_info, info);
                    self.state = InternalState::Empty;
                    return AssemblyState::ReadMore;
                }
                if info != previous_info {
                    log::warn!("transport: conflicting addresses, previous segment with {:?}, but received {:?}", previous_info, info);
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

        match self.buffer.get_mut(acc_length..new_length) {
            None => {
                log::warn!(
                    "transport buffer overflow with {} bytes to write",
                    data.len()
                );
                self.state = InternalState::Empty;
            }
            Some(dest) => {
                dest.copy_from_slice(data);
                if header.fin {
                    let count = self.count;
                    let info = FragmentInfo::new(count, info.source, info.broadcast);
                    self.count = self.count.wrapping_add(1);
                    self.state = InternalState::Complete(info, new_length)
                } else {
                    self.state = InternalState::Running(info, header, new_length)
                }
            }
        }
    }
}
