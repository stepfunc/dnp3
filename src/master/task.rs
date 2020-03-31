use crate::app::format::write;
use crate::app::header::ResponseFunction;
use crate::app::parse::parser::{ObjectParseError, Response};
use crate::app::sequence::Sequence;
use crate::error::Error;
use crate::transport::reader::{Fragment, Reader};
use crate::transport::writer::Writer;
use crate::util::cursor::{WriteCursor, WriteError};
use std::time::Duration;
use tokio::prelude::{AsyncRead, AsyncWrite};
use tokio::time::Instant;

#[derive(Copy, Clone, Debug)]
pub enum ResponseError {
    BadObjects(ObjectParseError),
}

impl std::convert::From<ObjectParseError> for ResponseError {
    fn from(err: ObjectParseError) -> Self {
        ResponseError::BadObjects(err)
    }
}

pub enum ResponseResult {
    /// the response was successfully processed
    Success,
    ///// run a new task - e.g. select then operate
    //Transition(Box<dyn MasterTask>),
}

pub trait MasterTask {
    fn is_read(&self) -> bool;
    fn format(&self, seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError>;
    fn handle(&self, response: Response) -> Result<ResponseResult, ResponseError>;
}

pub struct IntegrityPoll;

impl IntegrityPoll {
    pub fn create() -> Box<dyn MasterTask> {
        Box::new(IntegrityPoll {})
    }
}

impl MasterTask for IntegrityPoll {
    fn is_read(&self) -> bool {
        true
    }

    fn format(&self, seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        write::read_integrity(seq, cursor)
    }

    fn handle(&self, response: Response) -> Result<ResponseResult, ResponseError> {
        log::info!(
            "fir: {} fin: {} seq: {}",
            response.header.control.fir,
            response.header.control.fin,
            response.header.control.seq.value()
        );

        for _ in response.parse_objects()? {
            log::info!("got a header");
        }

        Ok(ResponseResult::Success)
    }
}

struct ResponseCount {
    count: usize,
}

impl ResponseCount {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    pub fn reset(&mut self) {
        self.count = 0
    }

    pub fn is_none(&self) -> bool {
        self.count == 0
    }

    pub fn increment(&mut self) {
        self.count += 1
    }
}

pub struct TaskRunner {
    seq: Sequence,
    reply_timeout: Duration,
    count: ResponseCount,
    buffer: [u8; 2048],
}

impl TaskRunner {
    pub fn new(reply_timeout: Duration) -> Self {
        Self {
            seq: Sequence::default(),
            reply_timeout,
            count: ResponseCount::new(),
            buffer: [0; 2048],
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum TaskError {
    Lower(Error),
    BadResponse(ResponseError),
    NeverReceivedFir,
    UnexpectedFir,
    BadSequence,
    ResponseTimeout,
    WriteError,
}

impl std::convert::From<WriteError> for TaskError {
    fn from(_: WriteError) -> Self {
        TaskError::WriteError
    }
}

impl std::convert::From<Error> for TaskError {
    fn from(err: Error) -> Self {
        TaskError::Lower(err)
    }
}

impl std::convert::From<tokio::time::Elapsed> for TaskError {
    fn from(_: tokio::time::Elapsed) -> Self {
        TaskError::ResponseTimeout
    }
}

impl std::convert::From<ResponseError> for TaskError {
    fn from(err: ResponseError) -> Self {
        TaskError::BadResponse(err)
    }
}

impl TaskRunner {
    async fn handle_unsolicited<T>(
        &mut self,
        io: &mut T,
        rsp: Response<'_>,
        writer: &mut Writer,
    ) -> Result<(), Error>
    where
        T: AsyncWrite + Unpin,
    {
        // TODO - actually handle it and write to correct destination
        if rsp.header.control.con {
            let mut cursor = WriteCursor::new(&mut self.buffer);
            write::confirm_unsolicited(rsp.header.control.seq, &mut cursor)?;
            writer.write(io, 1024, cursor.written()).await?;
        }

        Ok(())
    }

    async fn handle_non_read_response<T>(
        &mut self,
        _io: &mut T,
        _rsp: Response<'_>,
        _task: &dyn MasterTask,
        _writer: &mut Writer,
    ) -> Result<ResponseResult, TaskError>
    where
        T: AsyncWrite + Unpin,
    {
        Ok(ResponseResult::Success)
    }

    async fn handle_read_response<T>(
        &mut self,
        io: &mut T,
        rsp: Response<'_>,
        task: &dyn MasterTask,
        writer: &mut Writer,
    ) -> Result<ResponseResult, TaskError>
    where
        T: AsyncWrite + Unpin,
    {
        if rsp.header.control.seq.value() != self.seq.previous_value() {
            return Err(TaskError::BadSequence);
        }

        if rsp.header.control.fir && !self.count.is_none() {
            return Err(TaskError::UnexpectedFir);
        }

        if !rsp.header.control.fir && self.count.is_none() {
            return Err(TaskError::NeverReceivedFir);
        }

        if !rsp.header.control.fin && !rsp.header.control.con {
            log::warn!("received non-FIN response NOT requesting confirmation")
        }

        self.count.increment();

        // write a confirmation if required
        if rsp.header.control.con {
            let mut cursor = WriteCursor::new(&mut self.buffer);
            write::confirm_solicited(rsp.header.control.seq, &mut cursor)?;
            writer.write(io, 1024, cursor.written()).await?;
        }

        match task.handle(rsp)? {
            ResponseResult::Success => {
                self.seq.increment();
                Ok(ResponseResult::Success)
            }
        }
    }

    async fn handle_response<T>(
        &mut self,
        io: &mut T,
        rsp: Response<'_>,
        task: &dyn MasterTask,
        writer: &mut Writer,
    ) -> Result<ResponseResult, TaskError>
    where
        T: AsyncWrite + Unpin,
    {
        if task.is_read() {
            self.handle_read_response(io, rsp, task, writer).await
        } else {
            self.handle_non_read_response(io, rsp, task, writer).await
        }
    }

    pub async fn run<T>(
        &mut self,
        io: &mut T,
        task: &dyn MasterTask,
        writer: &mut Writer,
        reader: &mut Reader,
    ) -> Result<(), TaskError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        self.count.reset();
        // format the request
        let seq = self.seq.increment();
        let mut cursor = WriteCursor::new(&mut self.buffer);
        task.format(seq, &mut cursor)?;
        writer.write(io, 1024, cursor.written()).await?;

        let deadline = Instant::now() + self.reply_timeout;

        // now enter a loop to read responses
        loop {
            let fragment: Fragment = tokio::time::timeout_at(deadline, reader.read(io)).await??;
            match Response::parse(fragment.data) {
                Err(err) => log::warn!("error parsing response header: {:?}", err),
                Ok(response) => {
                    if response.header.function == ResponseFunction::Unsolicited {
                        self.handle_unsolicited(io, response, writer).await?;
                    } else {
                        match self.handle_response(io, response, task, writer).await? {
                            ResponseResult::Success => return Ok(()),
                        }
                    }
                }
            }
        }
    }
}
