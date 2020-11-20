use crate::link::header::FrameInfo;
use crate::transport::header::Header;
use crate::transport::Fragment;

#[derive(Copy, Clone)]
enum InternalState {
    Empty,
    // last address, header, and accumulated length
    Running(FrameInfo, Header, usize),
    // buffer contains an assembled ADU
    Complete(FrameInfo, usize),
}

pub(crate) enum AssemblyState {
    Complete,
    ReadMore,
}

pub(crate) struct Assembler {
    state: InternalState,
    buffer: [u8; 2048], // make this configurable and/or constant
}

impl Assembler {
    pub(crate) fn new() -> Self {
        Self {
            state: InternalState::Empty,
            buffer: [0; 2048],
        }
    }

    pub(crate) fn reset(&mut self) {
        self.state = InternalState::Empty;
    }

    pub(crate) fn peek(&self) -> Option<Fragment> {
        match self.state {
            InternalState::Complete(address, size) => Some(Fragment {
                address,
                data: &self.buffer[0..size],
            }),
            _ => None,
        }
    }

    pub(crate) fn assemble(
        &mut self,
        address: FrameInfo,
        header: Header,
        payload: &[u8],
    ) -> AssemblyState {
        // FIR always clears the state
        if header.fir {
            if let InternalState::Running(address, _, size) = self.state {
                log::warn!(
                    "transport: received FIR - dropping {} assembled bytes from {}",
                    size,
                    address.source
                );
            }
            self.state = InternalState::Empty;
        }

        match self.state {
            InternalState::Complete(_, _) => {
                self.state = InternalState::Empty;
                self.append(address, header, 0, payload);
            }
            InternalState::Empty => {
                // ignore non-FIR segments if there was no previous frame
                if !header.fir {
                    log::warn!(
                        "transport: ignoring non-FIR segment from {} with no previous FIR",
                        address.source
                    );
                    return AssemblyState::ReadMore;
                }
                self.append(address, header, 0, payload);
            }
            InternalState::Running(previous_address, previous_header, length) => {
                if header.seq.value() != previous_header.seq.next() {
                    log::warn!("transport: conflicting addresses, previous segment with {:?}, but received {:?}", previous_address, address);
                    self.state = InternalState::Empty;
                    return AssemblyState::ReadMore;
                }
                if address != previous_address {
                    log::warn!("transport: conflicting addresses, previous segment with {:?}, but received {:?}", previous_address, address);
                    self.state = InternalState::Empty;
                    return AssemblyState::ReadMore;
                }
                self.append(address, header, length, payload);
            }
        }

        match self.state {
            InternalState::Complete(_, _) => AssemblyState::Complete,
            _ => AssemblyState::ReadMore,
        }
    }

    fn append(&mut self, address: FrameInfo, header: Header, acc_length: usize, data: &[u8]) {
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
                    self.state = InternalState::Complete(address, new_length)
                } else {
                    self.state = InternalState::Running(address, header, new_length)
                }
            }
        }
    }
}
