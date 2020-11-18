use crate::app::enums::FunctionCode;
use crate::app::header::{Control, ResponseFunction, ResponseHeader, IIN, IIN1, IIN2};
use crate::app::parse::parser::ParsedFragment;
use crate::app::parse::DecodeLogLevel;
use crate::app::sequence::Sequence;
use crate::link::error::LinkError;
use crate::link::header::Address;
use crate::outstation::database::{DatabaseConfig, DatabaseHandle};
use crate::transport::{Fragment, ReaderType, WriterType};
use crate::util::cursor::WriteCursor;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::Duration;

pub(crate) struct Session {
    buffer: [u8; 2048],
    log_level: DecodeLogLevel,
    database: DatabaseHandle,
}

pub struct OutstationTask {
    session: Session,
    reader: ReaderType,
    writer: WriterType,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OutstationConfig {
    pub outstation_address: u16,
    pub master_address: Option<u16>,
    pub log_level: DecodeLogLevel,
    pub confirm_timeout: Duration,
}

impl OutstationConfig {
    pub fn new(
        outstation_address: u16,
        master_address: Option<u16>,
        log_level: DecodeLogLevel,
        confirm_timeout: Duration,
    ) -> Self {
        OutstationConfig {
            outstation_address,
            master_address,
            log_level,
            confirm_timeout,
        }
    }
}

impl OutstationTask {
    /// create an `OutstationTask` and return it along with a `DatabaseHandle` for updating it
    pub fn create(config: OutstationConfig, database: DatabaseConfig) -> (Self, DatabaseHandle) {
        let handle = DatabaseHandle::new(database);
        let (reader, writer) =
            crate::transport::create_transport_layer(false, config.outstation_address);
        let task = Self {
            session: Session::new(config.log_level, handle.clone()),
            reader,
            writer,
        };
        (task, handle)
    }

    /// run the outstation task asynchronously until a `LinkError` occurs
    pub async fn run<T>(&mut self, io: &mut T) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        self.session
            .run(io, &mut self.reader, &mut self.writer)
            .await
    }
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

impl Session {
    pub(crate) fn new(log_level: DecodeLogLevel, database: DatabaseHandle) -> Self {
        Self {
            buffer: [0; 2048],
            database,
            log_level,
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
        let request = match ParsedFragment::parse(self.log_level.receive(), fragment.data) {
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
                Ok(headers) => self.database.select(&headers),
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
            let info = self.database.write_response_headers(&mut cursor);

            let confirm = info.need_confirm();

            let iin2 = if fir { iin2 } else { IIN2::default() };
            let header = ResponseHeader::new(
                Control::response(seq, fir, info.complete, confirm),
                ResponseFunction::Response,
                IIN::new(info.unwritten.as_iin1(), iin2),
            );
            cursor.at_pos(0, |c| header.write(c))?;
            writer
                .write(self.log_level, io, 1, cursor.written())
                .await?;

            match self.wait_sol_confirm(io, reader, seq).await? {
                Confirm::Timeout => {
                    self.database.reset();
                    return Ok(());
                }
                Confirm::NewRequest => {
                    return Ok(());
                }
                Confirm::Yes => {}
            }

            self.database.clear_written_events();

            if info.complete {
                return Ok(());
            }

            fir = false;
            seq.increment();
        }
    }

    fn parse_confirm(
        log_level: DecodeLogLevel,
        fragment: Fragment,
        seq: Sequence,
    ) -> Option<Confirm> {
        let parsed = match ParsedFragment::parse(log_level.receive(), fragment.data) {
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
                        if let Some(result) = Self::parse_confirm(self.log_level, fragment, seq) {
                          return Ok(result);
                        }
                     }
                }
            }
        }

        Ok(Confirm::Timeout)
    }
}
