use crate::app::enums::FunctionCode;
use crate::app::header::{Control, ResponseFunction, ResponseHeader, IIN, IIN1, IIN2};
use crate::app::parse::parser::ParsedFragment;
use crate::app::parse::DecodeLogLevel;
use crate::app::sequence::Sequence;
use crate::link::error::LinkError;
use crate::link::header::Address;
use crate::transport::{Fragment, ReaderType, WriterType};
use crate::util::cursor::WriteCursor;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncRead, AsyncWrite};

/*
pub trait Update<T> {
    fn update(&mut self, value: &T, index: u16, options: UpdateOptions) -> bool;
}
*/

pub struct State {
    pub(crate) database: Arc<Mutex<crate::outstation::db::database::Database>>,
}

pub(crate) struct Task {
    buffer: [u8; 2048],
    state: State,
}

enum ResponseAction {
    Respond(Address, usize),
    BeginReadResponse(Sequence, IIN2),
}

enum Confirm {
    Yes,
    Timeout,
    NewRequest,
}

impl Task {
    pub(crate) fn new(database: Arc<Mutex<crate::outstation::db::database::Database>>) -> Self {
        Self {
            buffer: [0; 2048],
            state: State { database },
        }
    }

    pub(crate) async fn run<T>(
        &mut self,
        io: &mut T,
        reader: &mut ReaderType,
        writer: &mut WriterType,
    ) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        loop {
            // process any available request fragments
            if let Some(request) = reader.peek() {
                if let Some(action) = self.process_request(request) {
                    match action {
                        ResponseAction::Respond(address, num) => {
                            writer
                                .write(
                                    DecodeLogLevel::ObjectValues,
                                    io,
                                    address.source,
                                    &self.buffer[0..num],
                                )
                                .await?
                        }
                        ResponseAction::BeginReadResponse(seq, iin2) => {
                            self.respond_to_read(io, reader, writer, seq, iin2).await?;
                        }
                    }
                }
            }

            // what for data to be available, this is where we can select on other things
            reader.read(io).await?;
        }
    }

    fn process_request(&mut self, fragment: Fragment) -> Option<ResponseAction> {
        let request =
            match ParsedFragment::parse(DecodeLogLevel::ObjectValues.receive(), fragment.data) {
                // this is logged internally
                Err(_) => return None,
                Ok(x) => match x.to_request() {
                    Ok(request) => request,
                    Err(err) => {
                        log::error!("bad request: {}", err);
                        return None;
                    }
                },
            };

        if request.header.function == FunctionCode::Read {
            let iin = match request.objects {
                Err(_) => IIN2::PARAMETER_ERROR,
                Ok(headers) => self.state.database.lock().unwrap().select(&headers),
            };
            Some(ResponseAction::BeginReadResponse(
                request.header.control.seq,
                iin,
            ))
        } else {
            // here's where we'd call the non-read response processor
            let mut cursor = WriteCursor::new(self.buffer.as_mut());
            let header = ResponseHeader::new(
                request.header.control,
                ResponseFunction::Response,
                IIN::new(IIN1::default(), IIN2::NO_FUNC_CODE_SUPPORT),
            );
            let _ = header.write(&mut cursor);
            Some(ResponseAction::Respond(
                fragment.address,
                cursor.written().len(),
            ))
        }
    }

    async fn respond_to_read<T>(
        &mut self,
        io: &mut T,
        reader: &mut ReaderType,
        writer: &mut WriterType,
        mut seq: Sequence,
        iin2: IIN2,
    ) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let mut fir = true;

        loop {
            let mut cursor = WriteCursor::new(self.buffer.as_mut());
            cursor.skip(4)?;
            let info = self
                .state
                .database
                .lock()
                .unwrap()
                .write_response_headers(&mut cursor);
            let confirm = info.need_confirm();

            let iin2 = if fir { iin2 } else { IIN2::default() };
            let header = ResponseHeader::new(
                Control::response(seq, fir, info.complete, confirm),
                ResponseFunction::Response,
                IIN::new(info.unwritten.as_iin1(), iin2),
            );
            cursor.at_pos(0, |c| header.write(c))?;
            writer
                .write(DecodeLogLevel::ObjectValues, io, 1, cursor.written())
                .await?;

            match self.wait_sol_confirm(io, reader, seq).await? {
                Confirm::Timeout => {
                    self.state.database.lock().unwrap().reset();
                    return Ok(());
                }
                Confirm::NewRequest => {
                    return Ok(());
                }
                Confirm::Yes => {}
            }

            self.state.database.lock().unwrap().clear_written_events();

            if info.complete {
                return Ok(());
            }

            fir = false;
            seq.increment();
        }
    }

    fn parse_confirm(fragment: Fragment, seq: Sequence) -> Option<Confirm> {
        let parsed =
            match ParsedFragment::parse(DecodeLogLevel::ObjectValues.receive(), fragment.data) {
                Ok(parsed) => parsed,
                Err(_) => return None,
            };

        let request = match parsed.to_request() {
            Ok(request) => request,
            Err(err) => {
                log::warn!("bad request: {}", err);
                return None;
            }
        };

        if request.header.function != FunctionCode::Confirm {
            return Some(Confirm::NewRequest);
        }

        if request.header.control.seq != seq {
            log::warn!(
                "unexpected confirm seq: {}",
                request.header.control.seq.value()
            );
            return None;
        }

        Some(Confirm::Yes)
    }

    async fn wait_sol_confirm<T>(
        &mut self,
        io: &mut T,
        reader: &mut ReaderType,
        seq: Sequence,
    ) -> Result<Confirm, LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let mut delay = tokio::time::delay_for(tokio::time::Duration::from_secs(5));

        while !delay.is_elapsed() {
            tokio::select! {
                _ = &mut delay, if !delay.is_elapsed() => {
                    return Ok(Confirm::Timeout);
                }
                _ = reader.read(io) => {
                     if let Some(fragment) =  reader.peek() {
                        if let Some(result) = Self::parse_confirm(fragment, seq) {
                          return Ok(result);
                        }
                     }
                }
            }
        }

        Ok(Confirm::Timeout)
    }
}

/*
impl Update<Binary> for Database {
    fn update(&mut self, value: &Binary, index: u16, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<DoubleBitBinary> for Database {
    fn update(&mut self, value: &DoubleBitBinary, index: u16, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<BinaryOutputStatus> for Database {
    fn update(&mut self, value: &BinaryOutputStatus, index: u16, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<Counter> for Database {
    fn update(&mut self, value: &Counter, index: u16, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<FrozenCounter> for Database {
    fn update(&mut self, value: &FrozenCounter, index: u16, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<Analog> for Database {
    fn update(&mut self, value: &Analog, index: u16, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<AnalogOutputStatus> for Database {
    fn update(&mut self, value: &AnalogOutputStatus, index: u16, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}
*/
