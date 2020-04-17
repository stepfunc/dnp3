use crate::app::format::write;
use crate::app::parse::parser::{HeaderCollection, ParseLogLevel, ParsedFragment, Response};
use crate::app::sequence::Sequence;
use crate::master::request::{MasterRequest, RequestStatus};
use crate::transport::{ReaderType, WriterType};

use crate::app::header::ResponseHeader;
use crate::app::parse::error::ObjectParseError;
use crate::link::error::LinkError;
use crate::master::handlers::ResponseHandler;
use crate::util::cursor::{WriteCursor, WriteError};

use crate::app::gen::enums::FunctionCode;
use std::fmt::Formatter;
use std::time::Duration;
use tokio::prelude::{AsyncRead, AsyncWrite};

struct ResponseCount {
    count: usize,
}

impl ResponseCount {
    pub(crate) fn new() -> Self {
        Self { count: 0 }
    }

    pub(crate) fn reset(&mut self) {
        self.count = 0
    }

    pub(crate) fn is_none(&self) -> bool {
        self.count == 0
    }

    pub(crate) fn increment(&mut self) {
        self.count += 1
    }
}

pub struct RequestRunner {
    level: ParseLogLevel,
    response_timeout: Duration,
    count: ResponseCount,
    unsolicited_handler: Box<dyn ResponseHandler>,
    buffer: [u8; 2048],
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RequestError {
    Lower(LinkError),
    MalformedResponse(ObjectParseError),
    NeverReceivedFir,
    UnexpectedFir,
    MultiFragmentResponse,
    ResponseTimeout,
    WriteError,
}

impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            RequestError::Lower(_) => f.write_str("I/O error"),
            RequestError::MalformedResponse(err) => write!(f, "malformed response: {}", err),
            RequestError::NeverReceivedFir => {
                f.write_str("received non-FIR response before receiving FIR")
            }
            RequestError::UnexpectedFir => {
                f.write_str("received FIR bit after already receiving FIR bit")
            }
            RequestError::MultiFragmentResponse => {
                f.write_str("received unexpected multi-fragment response")
            }
            RequestError::ResponseTimeout => f.write_str("no response received within timeout"),
            RequestError::WriteError => {
                f.write_str("unable to serialize the task's request (insufficient buffer space)")
            }
        }
    }
}

impl From<WriteError> for RequestError {
    fn from(_: WriteError) -> Self {
        RequestError::WriteError
    }
}

impl From<LinkError> for RequestError {
    fn from(err: LinkError) -> Self {
        RequestError::Lower(err)
    }
}

impl From<tokio::time::Elapsed> for RequestError {
    fn from(_: tokio::time::Elapsed) -> Self {
        RequestError::ResponseTimeout
    }
}

impl From<ObjectParseError> for RequestError {
    fn from(err: ObjectParseError) -> Self {
        RequestError::MalformedResponse(err)
    }
}

impl RequestRunner {
    pub fn new(
        level: ParseLogLevel,
        response_timeout: Duration,
        unsolicited_handler: Box<dyn ResponseHandler>,
    ) -> Self {
        Self {
            level,
            response_timeout,
            count: ResponseCount::new(),
            unsolicited_handler,
            buffer: [0; 2048],
        }
    }

    async fn confirm_solicited<T>(
        &mut self,
        io: &mut T,
        destination: u16,
        seq: Sequence,
        writer: &mut WriterType,
    ) -> Result<(), LinkError>
    where
        T: AsyncWrite + Unpin,
    {
        let mut cursor = WriteCursor::new(&mut self.buffer);
        write::confirm_solicited(seq, &mut cursor)?;
        writer
            .write(self.level, io, destination, cursor.written())
            .await?;
        Ok(())
    }

    async fn confirm_unsolicited<T>(
        &mut self,
        io: &mut T,
        destination: u16,
        seq: Sequence,
        writer: &mut WriterType,
    ) -> Result<(), LinkError>
    where
        T: AsyncWrite + Unpin,
    {
        let mut cursor = WriteCursor::new(&mut self.buffer);
        crate::app::format::write::confirm_unsolicited(seq, &mut cursor)?;

        writer
            .write(self.level, io, destination, cursor.written())
            .await?;
        Ok(())
    }

    pub(crate) async fn handle_unsolicited<T>(
        &mut self,
        source: u16,
        response: &Response<'_>,
        io: &mut T,
        writer: &mut WriterType,
    ) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        if let Ok(objects) = response.objects {
            self.unsolicited_handler
                .handle(source, response.header, objects);
        }

        if response.header.control.con {
            self.confirm_unsolicited(io, source, response.header.control.seq, writer)
                .await?;
        }

        Ok(())
    }

    async fn handle_non_read_response<T>(
        &mut self,
        io: &mut T,
        source: u16,
        header: ResponseHeader,
        objects: HeaderCollection<'_>,
        task: &mut MasterRequest,
        writer: &mut WriterType,
    ) -> Result<RequestStatus, RequestError>
    where
        T: AsyncWrite + Unpin,
    {
        if !(header.control.is_fir_and_fin()) {
            return Err(RequestError::MultiFragmentResponse);
        }

        // non-read responses REALLY shouldn't request confirmation
        // but we'll confirm them if requested and log
        if header.control.con {
            log::warn!("received response requesting confirmation to non-read request");
            self.confirm_solicited(io, task.session.destination(), header.control.seq, writer)
                .await?;
        }

        Ok(task
            .details
            .handle(&mut task.session, source, header, objects))
    }

    async fn handle_read_response<T>(
        &mut self,
        io: &mut T,
        source: u16,
        header: ResponseHeader,
        objects: HeaderCollection<'_>,
        task: &mut MasterRequest,
        writer: &mut WriterType,
    ) -> Result<RequestStatus, RequestError>
    where
        T: AsyncWrite + Unpin,
    {
        if header.control.fir && !self.count.is_none() {
            return Err(RequestError::UnexpectedFir);
        }

        if !header.control.fir && self.count.is_none() {
            return Err(RequestError::NeverReceivedFir);
        }

        if !header.control.fin && !header.control.con {
            log::warn!("received non-FIN response NOT requesting confirmation")
        }

        self.count.increment();

        // write a confirmation if required
        if header.control.con {
            self.confirm_solicited(io, task.session.destination(), header.control.seq, writer)
                .await?;
        }

        let status = task
            .details
            .handle(&mut task.session, source, header, objects);

        if !header.control.fin {
            task.session.increment_seq();
        }

        Ok(status)
    }

    async fn handle_response<T>(
        &mut self,
        io: &mut T,
        source: u16,
        response: &Response<'_>,
        task: &mut MasterRequest,
        writer: &mut WriterType,
    ) -> Result<RequestStatus, RequestError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        if response.header.unsolicited {
            self.handle_unsolicited(source, response, io, writer)
                .await?;
            return Ok(RequestStatus::ContinueWaiting);
        }

        if source != task.session.destination() {
            log::warn!(
                "Received unexpected solicited response from address: {}",
                source
            );
            return Ok(RequestStatus::ContinueWaiting);
        }

        // this allows us to detect things like RESTART, NEED_TIME, and EVENT_BUFFER_OVERFLOW
        task.session.process_response_iin(response.header.iin);

        if response.header.control.seq.value() != task.session.previous_seq() {
            log::warn!(
                "response with seq: {} doesn't match expected seq: {}",
                response.header.control.seq.value(),
                task.session.previous_seq()
            );
            return Ok(RequestStatus::ContinueWaiting);
        }

        // if we can't parse a response, this is a TaskError
        let objects = response.objects?;

        if task.details.function() == FunctionCode::Read {
            self.handle_read_response(io, source, response.header, objects, task, writer)
                .await
        } else {
            self.handle_non_read_response(io, source, response.header, objects, task, writer)
                .await
        }
    }

    async fn send_request<T>(
        &mut self,
        io: &mut T,
        task: &mut MasterRequest,
        writer: &mut WriterType,
    ) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        // format the request
        let seq = task.session.increment_seq();
        let mut cursor = WriteCursor::new(&mut self.buffer);
        task.details.format(seq, &mut cursor)?;
        writer
            .write(self.level, io, task.session.destination(), cursor.written())
            .await
    }

    pub async fn run<T>(
        &mut self,
        io: &mut T,
        task: &mut MasterRequest,
        writer: &mut WriterType,
        reader: &mut ReaderType,
    ) -> Result<(), RequestError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let result = self.run_impl(io, task, writer, reader).await;

        task.details.on_complete(result);

        result
    }

    async fn run_impl<T>(
        &mut self,
        io: &mut T,
        task: &mut MasterRequest,
        writer: &mut WriterType,
        reader: &mut ReaderType,
    ) -> Result<(), RequestError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        self.count.reset();

        self.send_request(io, task, writer).await?;
        let mut deadline = crate::util::timeout::Timeout::from_now(self.response_timeout);

        // now enter a loop to read responses
        loop {
            tokio::time::timeout_at(deadline.value, reader.read(io)).await??;

            if let Some(fragment) = reader.peek() {
                if let Ok(parsed) = ParsedFragment::parse(self.level.receive(), fragment.data) {
                    match parsed.to_response() {
                        Err(err) => log::warn!("{}", err),
                        Ok(response) => {
                            match self
                                .handle_response(
                                    io,
                                    fragment.address.source,
                                    &response,
                                    task,
                                    writer,
                                )
                                .await?
                            {
                                // we're done
                                RequestStatus::Complete => return Ok(()),
                                // go to next iteration of the loop without updating the timeout
                                RequestStatus::ContinueWaiting => continue,
                                // go to next iteration of the loop, but update the timeout for another response
                                RequestStatus::ReadNextResponse => {
                                    deadline.extend(self.response_timeout)
                                }
                                // format the request and go through the whole cycle again with a new timeout
                                RequestStatus::ExecuteNextStep => {
                                    self.send_request(io, task, writer).await?;
                                    deadline.extend(self.response_timeout)
                                }
                            }
                        }
                    }
                };
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::link::header::Address;
    use crate::master::handlers::NullReadHandler;
    use crate::master::session::Session;
    use crate::master::types::{Classes, EventClasses, ReadRequest};
    use crate::transport::mocks::{MockReader, MockWriter};
    use tokio_test::io::Builder;

    #[test]
    fn performs_multi_fragmented_class_scan() {
        let session = Session::new(1024);

        let mut task = session.read(
            ReadRequest::ClassScan(Classes::new(false, EventClasses::new(true, false, false))),
            NullReadHandler::create(),
        );

        let mut runner = RequestRunner::new(
            ParseLogLevel::Nothing,
            Duration::from_secs(1),
            NullReadHandler::create(),
        );

        let mut io = Builder::new()
            .write(&[0xC0, 0x01, 0x3C, 0x02, 0x06])
            // FIR=1, FIN=0, CON=1, SEQ = 0
            .read(&[0xA0, 0x81, 0x00, 0x00])
            // confirm
            .write(&[0xC0, 0x00])
            // FIR=0, FIN=0, CON=1, SEQ = 1
            .read(&[0x21, 0x81, 0x00, 0x00])
            // confirm
            .write(&[0xC1, 0x00])
            // FIR=0, FIN=1, CON=0, SEQ = 2
            .read(&[0x42, 0x81, 0x00, 0x00])
            .build();

        let mut writer = MockWriter::mock();
        let mut reader = MockReader::mock(Address::new(1, 1024));
        tokio_test::block_on(runner.run(&mut io, &mut task, &mut writer, &mut reader)).unwrap();
        assert_eq!(session.previous_seq(), 2);
    }
}
