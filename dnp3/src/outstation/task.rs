use crate::app::enums::FunctionCode;
use crate::app::header::{Control, ResponseFunction, ResponseHeader, IIN, IIN1, IIN2};
use crate::app::parse::parser::Request;
use crate::app::parse::DecodeLogLevel;
use crate::app::sequence::Sequence;
use crate::link::error::LinkError;
use crate::outstation::database::{DatabaseConfig, DatabaseHandle};
use crate::transport::{TransportReader, TransportType, TransportWriter};

use crate::entry::LinkAddress;
use crate::link::header::AnyAddress;
use crate::util::buffer::Buffer;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::Duration;

pub(crate) struct Session {
    level: DecodeLogLevel,
    tx_buffer: Buffer,
    database: DatabaseHandle,
}

pub struct OutstationTask {
    session: Session,
    reader: TransportReader,
    writer: TransportWriter,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OutstationConfig {
    pub tx_buffer_size: usize,
    pub outstation_address: LinkAddress,
    pub master_address: Option<u16>,
    pub log_level: DecodeLogLevel,
    pub confirm_timeout: Duration,
}

impl OutstationConfig {
    pub fn new(
        tx_buffer_size: usize,
        outstation_address: LinkAddress,
        master_address: Option<u16>,
        log_level: DecodeLogLevel,
        confirm_timeout: Duration,
    ) -> Self {
        OutstationConfig {
            tx_buffer_size,
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
        let (reader, writer) = crate::transport::create_transport_layer(
            TransportType::Outstation,
            config.outstation_address,
        );
        let task = Self {
            session: Session::new(config.log_level, config.tx_buffer_size, handle.clone()),
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
    Respond(usize),
    BeginReadResponse(Sequence, IIN2),
}

enum Confirm {
    Yes,
    Timeout,
    NewRequest,
}

impl Session {
    pub(crate) const DEFAULT_TX_BUFFER_SIZE: usize = 2048;
    pub(crate) const MIN_TX_BUFFER_SIZE: usize = 249; // 1 link frame

    pub(crate) fn new(
        level: DecodeLogLevel,
        tx_buffer_size: usize,
        database: DatabaseHandle,
    ) -> Self {
        Self {
            level,
            tx_buffer: Buffer::new(tx_buffer_size.min(Self::MIN_TX_BUFFER_SIZE)),
            database,
        }
    }

    pub(crate) async fn run<T>(
        &mut self,
        io: &mut T,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
    ) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        loop {
            // process any available request fragments
            if let Some((address, request)) = reader.get_request(self.level) {
                if let Some(action) = self.process_request(request) {
                    match action {
                        ResponseAction::Respond(num) => {
                            writer
                                .write(io, self.level, address, self.tx_buffer.get(num).unwrap())
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

    fn process_request(&mut self, request: Request) -> Option<ResponseAction> {
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
            let mut cursor = self.tx_buffer.write_cursor();
            let header = ResponseHeader::new(
                request.header.control,
                ResponseFunction::Response,
                IIN::new(IIN1::default(), IIN2::NO_FUNC_CODE_SUPPORT),
            );
            let _ = header.write(&mut cursor);
            Some(ResponseAction::Respond(cursor.written().len()))
        }
    }

    async fn respond_to_read<T>(
        &mut self,
        io: &mut T,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
        mut seq: Sequence,
        iin2: IIN2,
    ) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let mut fir = true;

        loop {
            let mut cursor = self.tx_buffer.write_cursor();
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
                .write(io, self.level, AnyAddress::from(1), cursor.written())
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

    async fn wait_sol_confirm<T>(
        &mut self,
        io: &mut T,
        reader: &mut TransportReader,
        _seq: Sequence,
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
                     if let Some((_address, request)) =  reader.get_request(self.level) {
                        if request.header.function == FunctionCode::Confirm {
                            return Ok(Confirm::Yes);
                        }
                        else {
                            return Ok(Confirm::NewRequest);
                        }
                     }
                }
            }
        }

        Ok(Confirm::Timeout)
    }
}
