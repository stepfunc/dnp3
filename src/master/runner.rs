use crate::app::format::write;
use crate::app::parse::parser::{HeaderCollection, ParseLogLevel, ParsedFragment, Response};
use crate::app::sequence::Sequence;
use crate::master::request::{MasterRequest, RequestStatus};
use crate::transport::{ReaderType, WriterType};

use crate::app::header::ResponseHeader;
use crate::app::parse::error::ObjectParseError;
use crate::link::error::LinkError;
use crate::util::cursor::{WriteCursor, WriteError};

use crate::app::gen::enums::FunctionCode;
use crate::master::association::{Next, NoSession, SessionMap};
use crate::master::types::CommandHeader;
use std::fmt::Formatter;
use std::ops::Add;
use std::time::{Duration, Instant};
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

pub(crate) enum Message {
    Command(Vec<CommandHeader>),
}

#[derive(Clone)]
pub struct MasterHandle {
    sender: tokio::sync::mpsc::Sender<Message>,
}

impl MasterHandle {
    pub(crate) fn new(sender: tokio::sync::mpsc::Sender<Message>) -> Self {
        Self { sender }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Shutdown;

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum RunError {
    Link(LinkError),
    Shutdown,
}

pub(crate) struct Runner {
    level: ParseLogLevel,
    response_timeout: Duration,
    count: ResponseCount,
    sessions: SessionMap,
    user_queue: tokio::sync::mpsc::Receiver<Message>,
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
    SessionRemoved(u16),
    Shutdown,
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
            RequestError::Shutdown => {
                f.write_str("the master was shutdown while executing the task")
            }
            RequestError::SessionRemoved(x) => write!(f, "session removed: {}", x),
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

impl From<LinkError> for RunError {
    fn from(err: LinkError) -> Self {
        RunError::Link(err)
    }
}

impl From<Shutdown> for RunError {
    fn from(_: Shutdown) -> Self {
        RunError::Shutdown
    }
}

impl From<Shutdown> for RequestError {
    fn from(_: Shutdown) -> Self {
        RequestError::Shutdown
    }
}

impl From<NoSession> for RequestError {
    fn from(x: NoSession) -> Self {
        RequestError::SessionRemoved(x.address)
    }
}

impl Runner {
    pub(crate) fn new(
        level: ParseLogLevel,
        response_timeout: Duration,
        sessions: SessionMap,
        user_queue: tokio::sync::mpsc::Receiver<Message>,
    ) -> Self {
        Self {
            level,
            response_timeout,
            sessions,
            count: ResponseCount::new(),
            user_queue,
            buffer: [0; 2048],
        }
    }

    pub(crate) fn reset(&mut self) {
        self.sessions.reset()
    }

    async fn idle_until<T>(
        &mut self,
        instant: Instant,
        io: &mut T,
        writer: &mut WriterType,
        reader: &mut ReaderType,
    ) -> Result<(), RunError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        loop {
            tokio::select! {
                result = self.process_message() => {
                   result?;
                }
                result = reader.read(io) => {
                   result?;
                   self.handle_fragment_while_idle(io, writer, reader).await?;
                }
                _ = tokio::time::delay_until(tokio::time::Instant::from_std(instant)) => {
                   return Ok(());
                }
            }
        }
    }

    async fn idle_forever<T>(
        &mut self,
        io: &mut T,
        writer: &mut WriterType,
        reader: &mut ReaderType,
    ) -> Result<(), RunError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        loop {
            tokio::select! {
                result = self.process_message() => {
                   result?;
                }
                result = reader.read(io) => {
                   result?;
                   self.handle_fragment_while_idle(io, writer, reader).await?;
                }
            }
        }
    }

    async fn handle_fragment_while_idle<T>(
        &mut self,
        io: &mut T,
        writer: &mut WriterType,
        reader: &mut ReaderType,
    ) -> Result<(), RunError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        if let Some(fragment) = reader.peek() {
            if let Ok(parsed) = ParsedFragment::parse(self.level.receive(), fragment.data) {
                match parsed.to_response() {
                    Err(err) => log::warn!("{}", err),
                    Ok(response) => {
                        if response.header.unsolicited {
                            self.handle_unsolicited(fragment.address.source, &response, io, writer)
                                .await?;
                        } else {
                            log::warn!(
                                "unexpected response with sequence: {}",
                                response.header.control.seq.value()
                            )
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn process_message(&mut self) -> Result<(), Shutdown> {
        match self.user_queue.recv().await {
            Some(x) => {
                match x {
                    // TODO - handle real messages!
                    Message::Command(_headers) => {
                        log::warn!("received command headers!");
                    }
                }
                Ok(())
            }
            None => Err(Shutdown),
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

    async fn handle_unsolicited<T>(
        &mut self,
        source: u16,
        response: &Response<'_>,
        io: &mut T,
        writer: &mut WriterType,
    ) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let session = match self.sessions.get_mut(source).ok() {
            Some(session) => session,
            None => {
                log::warn!(
                    "received unsolicited response from unknown address: {}",
                    source
                );
                return Ok(());
            }
        };

        if let Ok(objects) = response.objects {
            session.handle_response(response.header, objects);
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
        task: &mut MasterRequest,
        header: ResponseHeader,
        objects: HeaderCollection<'_>,
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
            self.confirm_solicited(io, task.address, header.control.seq, writer)
                .await?;
        }

        Ok(task.details.handle(
            self.sessions.get_mut(task.address)?,
            task.address,
            header,
            objects,
        ))
    }

    async fn handle_read_response<T>(
        &mut self,
        io: &mut T,
        task: &mut MasterRequest,
        header: ResponseHeader,
        objects: HeaderCollection<'_>,
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
            self.confirm_solicited(io, task.address, header.control.seq, writer)
                .await?;
        }

        let status = task.details.handle(
            self.sessions.get_mut(task.address)?,
            task.address,
            header,
            objects,
        );

        if !header.control.fin {
            self.sessions.get_mut(task.address)?.increment_seq();
        }

        Ok(status)
    }

    async fn handle_response<T>(
        &mut self,
        io: &mut T,
        task: &mut MasterRequest,
        source: u16,
        response: &Response<'_>,
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

        if source != task.address {
            log::warn!(
                "Received unexpected solicited response from address: {}",
                source
            );
            return Ok(RequestStatus::ContinueWaiting);
        }

        // this allows us to detect things like RESTART, NEED_TIME, and EVENT_BUFFER_OVERFLOW
        self.sessions
            .get_mut(task.address)?
            .process_response_iin(response.header.iin);

        if response.header.control.seq.value() != self.sessions.get(task.address)?.previous_seq() {
            log::warn!(
                "response with seq: {} doesn't match expected seq: {}",
                response.header.control.seq.value(),
                self.sessions.get(task.address)?.previous_seq()
            );
            return Ok(RequestStatus::ContinueWaiting);
        }

        // if we can't parse a response, this is a TaskError
        let objects = response.objects?;

        if task.details.function() == FunctionCode::Read {
            self.handle_read_response(io, task, response.header, objects, writer)
                .await
        } else {
            self.handle_non_read_response(io, task, response.header, objects, writer)
                .await
        }
    }

    async fn send_request<T>(
        &mut self,
        io: &mut T,
        task: &mut MasterRequest,
        writer: &mut WriterType,
    ) -> Result<(), RequestError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        // format the request
        let seq = self.sessions.get_mut(task.address)?.increment_seq();
        let mut cursor = WriteCursor::new(&mut self.buffer);
        task.details.format(seq, &mut cursor)?;
        writer
            .write(self.level, io, task.address, cursor.written())
            .await?;
        Ok(())
    }

    pub(crate) async fn delay_for(&mut self, duration: Duration) -> Result<(), Shutdown> {
        let deadline = Instant::now().add(duration);

        loop {
            tokio::select! {
                // TODO - fail incoming commands since no connection
                result = self.process_message() => {
                   result?;
                }
                _ = tokio::time::delay_until(tokio::time::Instant::from_std(deadline)) => {
                   return Ok(());
                }
            }
        }
    }

    pub(crate) async fn run<T>(
        &mut self,
        io: &mut T,
        writer: &mut WriterType,
        reader: &mut ReaderType,
    ) -> RunError
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        loop {
            let result = match self.sessions.next_task() {
                Next::Now(mut task) => self.run_task(io, &mut task, writer, reader).await,
                Next::NotBefore(time) => self.idle_until(time, io, writer, reader).await,
                Next::None => self.idle_forever(io, writer, reader).await,
            };

            if let Err(err) = result {
                return err;
            }
        }
    }

    async fn run_task<T>(
        &mut self,
        io: &mut T,
        task: &mut MasterRequest,
        writer: &mut WriterType,
        reader: &mut ReaderType,
    ) -> Result<(), RunError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let result = self.run_impl(io, task, writer, reader).await;

        task.details.on_complete(result);

        match result {
            Err(RequestError::Lower(err)) => Err(RunError::Link(err)),
            Err(RequestError::Shutdown) => Err(RunError::Shutdown),
            _ => Ok(()),
        }
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
            tokio::select! {
                _ = tokio::time::delay_until(deadline.value) => {
                     return Err(RequestError::ResponseTimeout);
                }
                x = reader.read(io)  => {
                    x?;
                }
                y = self.process_message() => {
                    y?;
                    continue;
                }
            }

            match self.process_response(io, task, writer, reader).await? {
                // we're done
                RequestStatus::Complete => return Ok(()),
                // go to next iteration of the loop without updating the timeout
                RequestStatus::ContinueWaiting => continue,
                // go to next iteration of the loop, but update the timeout for another response
                RequestStatus::ReadNextResponse => deadline.extend(self.response_timeout),
                // format the request and go through the whole cycle again with a new timeout
                RequestStatus::ExecuteNextStep => {
                    self.send_request(io, task, writer).await?;
                    deadline.extend(self.response_timeout)
                }
            }
        }
    }

    async fn process_response<T>(
        &mut self,
        io: &mut T,
        task: &mut MasterRequest,
        writer: &mut WriterType,
        reader: &mut ReaderType,
    ) -> Result<RequestStatus, RequestError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        if let Some(fragment) = reader.peek() {
            if let Ok(parsed) = ParsedFragment::parse(self.level.receive(), fragment.data) {
                match parsed.to_response() {
                    Err(err) => log::warn!("{}", err),
                    Ok(response) => {
                        return self
                            .handle_response(io, task, fragment.address.source, &response, writer)
                            .await
                    }
                }
            };
        }
        Ok(RequestStatus::ContinueWaiting)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::link::header::Address;
    use crate::master::association::{Association, AssociationConfig};
    use crate::master::handlers::NullHandler;
    use crate::transport::mocks::{MockReader, MockWriter};
    use tokio_test::io::Builder;

    #[tokio::test]
    async fn performs_startup_sequence_with_device_restart_asserted() {
        let map = SessionMap::single(Association::new(
            1024,
            AssociationConfig::default(),
            NullHandler::boxed(),
        ));

        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let mut runner = Runner::new(ParseLogLevel::Nothing, Duration::from_secs(1), map, rx);

        let (mut io, _handle) = Builder::new()
            // disable unsolicited
            .write(&[
                0xC0, 0x15, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06,
            ])
            // response w/ DEVICE_RESTART asserted
            .read(&[0xC0, 0x81, 0x80, 0x00])
            // clear the restart bit
            .write(&[0xC1, 0x02, 0x50, 0x01, 0x00, 0x07, 0x07, 0x00])
            // response w/ DEVICE_RESTART cleared
            .read(&[0xC1, 0x81, 0x00, 0x00])
            // integrity poll
            .write(&[
                0xC2, 0x01, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06, 0x3C, 0x01, 0x06,
            ])
            // response
            .read(&[0xC2, 0x81, 0x00, 0x00])
            // enable unsolicited
            .write(&[
                0xC3, 0x14, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06,
            ])
            // response
            .read(&[0xC3, 0x81, 0x00, 0x00])
            .build_with_handle();

        let mut writer = MockWriter::mock();
        let mut reader = MockReader::mock(Address::new(1, 1024));

        let mut task = tokio_test::task::spawn(runner.run(&mut io, &mut writer, &mut reader));

        tokio_test::assert_pending!(task.poll());
        drop(tx); // causes the task to shutdown
        tokio_test::assert_ready_eq!(task.poll(), RunError::Shutdown);
        drop(task);
        assert_eq!(writer.num_writes(), 4);
        assert_eq!(reader.num_reads(), 4);
    }
}
