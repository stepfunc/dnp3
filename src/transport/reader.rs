use crate::error::Error;
use crate::link::header::Address;
use crate::link::parser::FramePayload;
use crate::transport::header::Header;
use crate::util::cursor::WriteError;
use tokio::prelude::{AsyncRead, AsyncWrite};

#[derive(Copy, Clone)]
enum State {
    Empty,
    // last address, header, and accumulated length
    Running(Address, Header, usize),
}
pub struct Reader {
    link: crate::link::layer::Layer,
    state: State,
    buffer: [u8; 2048], // make this configurable and/or constant
}

#[derive(Debug)]
pub struct Fragment<'a> {
    pub address: Address,
    pub data: &'a [u8],
}

impl Reader {
    pub fn new(is_master: bool, address: u16) -> Self {
        Self {
            link: crate::link::layer::Layer::new(is_master, address),
            state: State::Empty,
            buffer: [0; 2048],
        }
    }

    pub fn reset(&mut self) {
        self.state = State::Empty;
        self.link.reset();
    }

    pub async fn read<T>(&mut self, io: &mut T) -> Result<Fragment<'_>, Error>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let mut payload = FramePayload::new();

        loop {
            let address = self.link.read(io, &mut payload).await?;
            match payload.get() {
                [transport, data @ ..] => {
                    let header = Header::new(*transport);
                    if let Some(count) = self.assemble(address, header, data)? {
                        return Ok(Fragment {
                            address,
                            data: &self.buffer[0..count],
                        });
                    }
                }
                [] => log::warn!("received link data frame with no payload"),
            }
        }
    }

    fn append(
        &mut self,
        header: Header,
        address: Address,
        length: usize,
        data: &[u8],
    ) -> Result<usize, WriteError> {
        let remaining = self.buffer.len() - length;
        if data.len() > remaining {
            log::warn!(
                "transport buffer overflow, {} bytes remaining, {} byte to write",
                remaining,
                data.len()
            );
            self.state = State::Empty;
            return Err(WriteError);
        }
        let new_length = length + data.len();
        self.buffer[length..new_length].copy_from_slice(data);
        self.state = State::Running(address, header, new_length);
        Ok(new_length)
    }

    fn assemble(
        &mut self,
        address: Address,
        header: Header,
        payload: &[u8],
    ) -> Result<Option<usize>, Error> {
        // FIR always clears the state
        if header.fir {
            self.state = State::Empty;
        }

        let length: usize = match self.state {
            State::Running(previous_address, previous_header, length) => {
                if header.seq.value() != previous_header.seq.next() {
                    self.state = State::Empty;
                    return Ok(None);
                }
                if address != previous_address {
                    log::info!("transport: conflicting addresses, previous segment with {:?}, but received {:?}", previous_address, address);
                    self.state = State::Empty;
                    return Ok(None);
                }
                self.append(header, address, length, payload)?
            }
            State::Empty => {
                // ignore non-FIR segments if there was no previous frame
                if !header.fir {
                    return Ok(None);
                }
                self.append(header, address, 0, payload)?
            }
        };

        if header.fin {
            let ret = length;
            self.state = State::Empty;
            Ok(Some(ret))
        } else {
            Ok(None)
        }
    }
}
