use crate::app::format::write;
use crate::app::parse::parser::Response;
use crate::app::sequence::Sequence;
use crate::error::Error;
use crate::link::header::Address;
use crate::master::task::{MasterTask, ResponseError, ResponseResult};
use crate::transport::reader::Fragment;
use crate::transport::{ReaderType, WriterType};
use crate::util::cursor::{WriteCursor, WriteError};

use std::time::Duration;
use tokio::prelude::{AsyncRead, AsyncWrite};
use tokio::time::Instant;

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
    MultiFragmentResponse,
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
    async fn confirm_solicited<T>(
        &mut self,
        io: &mut T,
        destination: u16,
        seq: Sequence,
        writer: &mut WriterType,
    ) -> Result<(), Error>
    where
        T: AsyncWrite + Unpin,
    {
        let mut cursor = WriteCursor::new(&mut self.buffer);
        write::confirm_solicited(seq, &mut cursor)?;
        writer.write(io, destination, cursor.written()).await?;
        Ok(())
    }

    async fn confirm_unsolicited<T>(
        &mut self,
        io: &mut T,
        destination: u16,
        seq: Sequence,
        writer: &mut WriterType,
    ) -> Result<(), Error>
    where
        T: AsyncWrite + Unpin,
    {
        let mut cursor = WriteCursor::new(&mut self.buffer);
        write::confirm_unsolicited(seq, &mut cursor)?;
        writer.write(io, destination, cursor.written()).await?;
        Ok(())
    }

    async fn handle_unsolicited<T>(
        &mut self,
        io: &mut T,
        address: Address,
        rsp: Response<'_>,
        writer: &mut WriterType,
    ) -> Result<(), Error>
    where
        T: AsyncWrite + Unpin,
    {
        // TODO - invoke handling callback
        if rsp.header.control.con {
            self.confirm_unsolicited(io, address.source, rsp.header.control.seq, writer)
                .await?;
        }

        Ok(())
    }

    async fn handle_non_read_response<T>(
        &mut self,
        io: &mut T,
        rsp: Response<'_>,
        task: &MasterTask,
        writer: &mut WriterType,
    ) -> Result<ResponseResult, TaskError>
    where
        T: AsyncWrite + Unpin,
    {
        if rsp.header.control.seq.value() != self.seq.previous_value() {
            return Err(TaskError::BadSequence);
        }

        if !(rsp.header.control.fir && rsp.header.control.fin) {
            return Err(TaskError::MultiFragmentResponse);
        }

        // non-read responses REALLY shouldn't request confirmation
        // but we'll confirm them if requested and log
        if rsp.header.control.con {
            log::warn!("received response requesting confirmation to non-read request");
            self.confirm_solicited(io, task.destination, rsp.header.control.seq, writer)
                .await?;
        }

        Ok(task.details.handle(rsp)?)
    }

    async fn handle_read_response<T>(
        &mut self,
        io: &mut T,
        rsp: Response<'_>,
        task: &MasterTask,
        writer: &mut WriterType,
    ) -> Result<ResponseResult, TaskError>
    where
        T: AsyncWrite + Unpin,
    {
        // validate the sequence number
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
            writer.write(io, task.destination, cursor.written()).await?;
        }

        let result = task.details.handle(rsp)?;

        if let ResponseResult::Continue = result {
            self.seq.increment();
        }

        Ok(result)
    }

    async fn handle_response<T>(
        &mut self,
        io: &mut T,
        rsp: Response<'_>,
        task: &MasterTask,
        writer: &mut WriterType,
    ) -> Result<ResponseResult, TaskError>
    where
        T: AsyncWrite + Unpin,
    {
        if task.details.is_read_request() {
            self.handle_read_response(io, rsp, task, writer).await
        } else {
            self.handle_non_read_response(io, rsp, task, writer).await
        }
    }

    pub async fn run<T>(
        &mut self,
        io: &mut T,
        task: &MasterTask,
        writer: &mut WriterType,
        reader: &mut ReaderType,
    ) -> Result<(), TaskError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        self.count.reset();
        // format the request
        let seq = self.seq.increment();
        let mut cursor = WriteCursor::new(&mut self.buffer);
        task.details.format(seq, &mut cursor)?;
        writer.write(io, task.destination, cursor.written()).await?;

        let mut deadline = Instant::now() + self.reply_timeout;

        // now enter a loop to read responses
        loop {
            let fragment: Fragment = tokio::time::timeout_at(deadline, reader.read(io)).await??;
            match Response::parse(fragment.data) {
                Err(err) => log::warn!("error parsing response header: {:?}", err),
                Ok(response) => {
                    if response.header.unsolicited {
                        self.handle_unsolicited(io, fragment.address, response, writer)
                            .await?;
                    } else {
                        match self.handle_response(io, response, task, writer).await? {
                            ResponseResult::Complete => return Ok(()),
                            ResponseResult::Continue => {
                                // continue to next iteration of the loop, read another reply
                                deadline = Instant::now() + self.reply_timeout;
                            }
                        };
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::master::task::TaskDetails;
    use crate::master::types::ClassScan;
    use crate::transport::mocks::{MockReader, MockWriter};
    use tokio_test::io::Builder;

    #[test]
    fn performs_multi_fragmented_integrity_scan() {
        let task = MasterTask::new(1024, TaskDetails::ClassScan(ClassScan::integrity()));

        let mut runner = TaskRunner::new(Duration::from_secs(1));

        let mut io= Builder::new()
            .write(&[
                0xC0, 0x01, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06, 0x3C, 0x01, 0x06,
            ])
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
        tokio_test::block_on(runner.run(&mut io, &task, &mut writer, &mut reader)).unwrap();
    }
}
