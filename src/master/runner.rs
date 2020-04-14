use crate::app::format::write;
use crate::app::parse::parser::{HeaderCollection, ParseLogLevel, ParsedFragment, Response};
use crate::app::sequence::Sequence;
use crate::master::task::{MasterTask, TaskStatus};
use crate::transport::{ReaderType, WriterType};

use crate::app::header::ResponseHeader;
use crate::app::parse::error::ObjectParseError;
use crate::link::error::LinkError;
use crate::master::handlers::ResponseHandler;
use crate::master::unsolicited::UnsolicitedHandler;
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

pub struct TaskRunner {
    level: ParseLogLevel,
    seq: Sequence,
    reply_timeout: Duration,
    count: ResponseCount,
    unsolicited_handler: UnsolicitedHandler,
    buffer: [u8; 2048],
}

impl TaskRunner {
    pub fn new(
        level: ParseLogLevel,
        reply_timeout: Duration,
        unsolicited_handler: Box<dyn ResponseHandler>,
    ) -> Self {
        Self {
            level,
            seq: Sequence::default(),
            reply_timeout,
            count: ResponseCount::new(),
            unsolicited_handler: UnsolicitedHandler::new(unsolicited_handler),
            buffer: [0; 2048],
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TaskError {
    Lower(LinkError),
    MalformedResponse(ObjectParseError),
    NeverReceivedFir,
    UnexpectedFir,
    MultiFragmentResponse,
    ResponseTimeout,
    WriteError,
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            TaskError::Lower(_) => f.write_str("I/O error"),
            TaskError::MalformedResponse(err) => write!(f, "malformed response: {}", err),
            TaskError::NeverReceivedFir => {
                f.write_str("received non-FIR response before receiving FIR")
            }
            TaskError::UnexpectedFir => {
                f.write_str("received FIR bit after already receiving FIR bit")
            }
            TaskError::MultiFragmentResponse => {
                f.write_str("received unexpected multi-fragment response")
            }
            TaskError::ResponseTimeout => f.write_str("no response received within timeout"),
            TaskError::WriteError => {
                f.write_str("unable to serialize the task's request (insufficient buffer space)")
            }
        }
    }
}

impl From<WriteError> for TaskError {
    fn from(_: WriteError) -> Self {
        TaskError::WriteError
    }
}

impl From<LinkError> for TaskError {
    fn from(err: LinkError) -> Self {
        TaskError::Lower(err)
    }
}

impl From<tokio::time::Elapsed> for TaskError {
    fn from(_: tokio::time::Elapsed) -> Self {
        TaskError::ResponseTimeout
    }
}

impl From<ObjectParseError> for TaskError {
    fn from(err: ObjectParseError) -> Self {
        TaskError::MalformedResponse(err)
    }
}

impl TaskRunner {
    async fn confirm<T>(
        level: ParseLogLevel,
        io: &mut T,
        destination: u16,
        seq: Sequence,
        writer: &mut WriterType,
    ) -> Result<(), LinkError>
    where
        T: AsyncWrite + Unpin,
    {
        let mut buffer: [u8; 2] = [0; 2];
        let mut cursor = WriteCursor::new(&mut buffer);
        write::confirm_solicited(seq, &mut cursor)?;
        writer
            .write(level, io, destination, cursor.written())
            .await?;
        Ok(())
    }

    async fn handle_non_read_response<T>(
        &mut self,
        io: &mut T,
        source: u16,
        header: ResponseHeader,
        objects: HeaderCollection<'_>,
        task: &mut MasterTask,
        writer: &mut WriterType,
    ) -> Result<TaskStatus, TaskError>
    where
        T: AsyncWrite + Unpin,
    {
        if !(header.control.is_fir_and_fin()) {
            return Err(TaskError::MultiFragmentResponse);
        }

        // non-read responses REALLY shouldn't request confirmation
        // but we'll confirm them if requested and log
        if header.control.con {
            log::warn!("received response requesting confirmation to non-read request");
            Self::confirm(self.level, io, task.destination, header.control.seq, writer).await?;
        }

        Ok(task.details.handle(source, header, objects))
    }

    async fn handle_read_response<T>(
        &mut self,
        io: &mut T,
        source: u16,
        header: ResponseHeader,
        objects: HeaderCollection<'_>,
        task: &mut MasterTask,
        writer: &mut WriterType,
    ) -> Result<TaskStatus, TaskError>
    where
        T: AsyncWrite + Unpin,
    {
        if header.control.fir && !self.count.is_none() {
            return Err(TaskError::UnexpectedFir);
        }

        if !header.control.fir && self.count.is_none() {
            return Err(TaskError::NeverReceivedFir);
        }

        if !header.control.fin && !header.control.con {
            log::warn!("received non-FIN response NOT requesting confirmation")
        }

        self.count.increment();

        // write a confirmation if required
        if header.control.con {
            Self::confirm(self.level, io, task.destination, header.control.seq, writer).await?;
        }

        let status = task.details.handle(source, header, objects);

        if !header.control.fin {
            self.seq.increment();
        }

        Ok(status)
    }

    async fn handle_response<T>(
        &mut self,
        io: &mut T,
        source: u16,
        response: &Response<'_>,
        task: &mut MasterTask,
        writer: &mut WriterType,
    ) -> Result<TaskStatus, TaskError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        if response.header.unsolicited {
            self.unsolicited_handler
                .handle(self.level, source, response, io, writer)
                .await?;
            return Ok(TaskStatus::ContinueWaiting);
        }

        if response.header.control.seq.value() != self.seq.previous() {
            log::warn!(
                "response with seq: {} doesn't match expected seq: {}",
                response.header.control.seq.value(),
                self.seq.previous()
            );
            return Ok(TaskStatus::ContinueWaiting);
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
        task: &mut MasterTask,
        writer: &mut WriterType,
    ) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        // format the request
        let seq = self.seq.increment();
        let mut cursor = WriteCursor::new(&mut self.buffer);
        task.details.format(seq, &mut cursor)?;
        writer
            .write(self.level, io, task.destination, cursor.written())
            .await
    }

    pub async fn run<T>(
        &mut self,
        io: &mut T,
        task: &mut MasterTask,
        writer: &mut WriterType,
        reader: &mut ReaderType,
    ) -> Result<(), TaskError>
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
        task: &mut MasterTask,
        writer: &mut WriterType,
        reader: &mut ReaderType,
    ) -> Result<(), TaskError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        self.count.reset();

        self.send_request(io, task, writer).await?;
        let mut deadline = crate::util::timeout::Timeout::from_now(self.reply_timeout);

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
                                TaskStatus::Complete => return Ok(()),
                                // go to next iteration of the loop without updating the timeout
                                TaskStatus::ContinueWaiting => continue,
                                // go to next iteration of the loop, but update the timeout for another response
                                TaskStatus::ReadNextResponse => deadline.extend(self.reply_timeout),
                                // format the request and go through the whole cycle again with a new timeout
                                TaskStatus::ExecuteNextStep => {
                                    self.send_request(io, task, writer).await?;
                                    deadline.extend(self.reply_timeout)
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
    use crate::master::handlers::NullReadHandler;
    use crate::master::types::{Classes, EventClasses, ReadRequest};
    use crate::transport::mocks::{MockReader, MockWriter};
    use tokio_test::io::Builder;

    #[test]
    fn performs_multi_fragmented_class_scan() {
        let mut task = MasterTask::read(
            1024,
            ReadRequest::ClassScan(Classes::new(false, EventClasses::new(true, false, false))),
            NullReadHandler::create(),
        );

        let mut runner = TaskRunner::new(
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
        let mut reader = MockReader::mock();
        tokio_test::block_on(runner.run(&mut io, &mut task, &mut writer, &mut reader)).unwrap();
    }
}
