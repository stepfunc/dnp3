use crate::app::format::write;
use crate::app::parse::parser::{ParseLogLevel, ParsedFragment, Response};
use crate::app::sequence::Sequence;
use crate::master::task::{
    NonReadTask, NonReadTaskStatus, ReadTask, RequestWriter, Task, TaskType,
};
use crate::transport::{ReaderType, WriterType};

use crate::app::header::Control;
use crate::app::parse::error::ObjectParseError;
use crate::link::error::LinkError;
use crate::util::cursor::{WriteCursor, WriteError};

use crate::app::format::write::start_request;
use crate::master::association::{AssociationMap, Next, NoSession};
use crate::master::handlers::{CallbackOnce, CommandCallback, CommandResult};
use crate::master::types::{CommandError, CommandHeader};

use crate::util::timeout::Timeout;
use std::collections::VecDeque;
use std::fmt::Formatter;
use std::ops::Add;
use std::time::{Duration, Instant};
use tokio::prelude::{AsyncRead, AsyncWrite};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CommandMode {
    DirectOperate,
    SelectBeforeOperate,
}

pub(crate) enum Message {
    Command(u16, CommandMode, Vec<CommandHeader>, CommandCallback),
}

impl Message {
    pub(crate) fn on_send_failure(self) {
        match self {
            Message::Command(_, _, _, callback) => {
                callback.complete(Err(TaskError::Shutdown.into()))
            }
        }
    }
}

#[derive(Clone)]
pub struct MasterHandle {
    sender: tokio::sync::mpsc::Sender<Message>,
}

impl MasterHandle {
    pub(crate) fn new(sender: tokio::sync::mpsc::Sender<Message>) -> Self {
        Self { sender }
    }

    pub async fn operate(
        &mut self,
        address: u16,
        mode: CommandMode,
        headers: Vec<CommandHeader>,
    ) -> CommandResult {
        let (tx, rx) = tokio::sync::oneshot::channel::<CommandResult>();
        self.send_operate_message(mode, address, headers, CallbackOnce::OneShot(tx))
            .await;
        rx.await?
    }

    pub async fn operate_cb<F>(
        &mut self,
        address: u16,
        mode: CommandMode,
        headers: Vec<CommandHeader>,
        callback: F,
    ) where
        F: FnOnce(CommandResult) -> () + Send + Sync + 'static,
    {
        self.send_operate_message(
            mode,
            address,
            headers,
            CallbackOnce::BoxedFn(Box::new(callback)),
        )
        .await;
    }

    async fn send_operate_message(
        &mut self,
        mode: CommandMode,
        address: u16,
        headers: Vec<CommandHeader>,
        callback: CommandCallback,
    ) {
        if let Err(tokio::sync::mpsc::error::SendError(msg)) = self
            .sender
            .send(Message::Command(address, mode, headers, callback))
            .await
        {
            msg.on_send_failure();
        }
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for CommandError {
    fn from(_: tokio::sync::oneshot::error::RecvError) -> Self {
        CommandError::Task(TaskError::Shutdown)
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
    timeout: Timeout,
    associations: AssociationMap,
    user_queue: tokio::sync::mpsc::Receiver<Message>,
    command_queue: VecDeque<Task>,
    buffer: [u8; 2048],
}

/// Errors that can occur while executing a master task
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TaskError {
    /// An error occurred at the link or transport level
    Lower(LinkError),
    /// A response to the task's request was malformed
    MalformedResponse(ObjectParseError),
    /// Non-final response not requesting confirmation
    NonFinWithoutCon,
    /// Received a non-FIR response when expecting the FIR bit
    NeverReceivedFir,
    /// Received FIR bit after already receiving FIR
    UnexpectedFir,
    /// Received a multi-fragmented response when expecting FIR/FIN
    MultiFragmentResponse,
    /// The response timed-out
    ResponseTimeout,
    /// Insufficient buffer space to serialize the request
    WriteError,
    /// The requested association does not exist (not configured)
    NoSuchAssociation(u16),
    /// There is not connection at the transport level
    NoConnection,
    /// The master was shutdown
    Shutdown,
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            TaskError::Lower(_) => f.write_str("I/O error"),
            TaskError::MalformedResponse(err) => write!(f, "malformed response: {}", err),
            TaskError::NonFinWithoutCon => {
                f.write_str("outstation responses with FIN == 0 must request confirmation")
            }
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
            TaskError::Shutdown => f.write_str("the master was shutdown while executing the task"),
            TaskError::NoConnection => f.write_str("no connection"),
            TaskError::NoSuchAssociation(x) => write!(f, "no association with address: {}", x),
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

impl From<Shutdown> for TaskError {
    fn from(_: Shutdown) -> Self {
        TaskError::Shutdown
    }
}

impl From<NoSession> for TaskError {
    fn from(x: NoSession) -> Self {
        TaskError::NoSuchAssociation(x.address)
    }
}

enum ReadResponseAction {
    Ignore,
    ReadNext,
    Complete,
}

impl Runner {
    pub(crate) fn new(
        level: ParseLogLevel,
        response_timeout: Timeout,
        associations: AssociationMap,
        user_queue: tokio::sync::mpsc::Receiver<Message>,
    ) -> Self {
        Self {
            level,
            timeout: response_timeout,
            associations,
            user_queue,
            command_queue: VecDeque::new(),
            buffer: [0; 2048],
        }
    }

    pub(crate) fn reset(&mut self) {
        self.associations.reset()
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
                result = self.process_message_while_connected() => {
                   if result? {
                       // we need to recheck the tasks
                       return Ok(());
                   }
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
                result = self.process_message_while_connected() => {
                   if result? {
                       // we need to recheck the tasks
                       return Ok(());
                   }
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
        let fragment = match reader.peek() {
            None => return Ok(()),
            Some(x) => x,
        };

        let parsed = match ParsedFragment::parse(self.level.receive(), fragment.data).ok() {
            None => return Ok(()),
            Some(x) => x,
        };

        let response = match parsed.to_response() {
            Err(err) => {
                log::warn!("{}", err);
                return Ok(());
            }
            Ok(x) => x,
        };

        if response.header.unsolicited {
            self.handle_unsolicited(fragment.address.source, &response, io, writer)
                .await?;
        } else {
            log::warn!(
                "unexpected response with sequence: {}",
                response.header.control.seq.value()
            )
        }

        Ok(())
    }

    async fn process_message_while_connected(&mut self) -> Result<bool, Shutdown> {
        match self.user_queue.recv().await {
            Some(x) => match x {
                Message::Command(address, mode, headers, handler) => {
                    match self.associations.get(address).ok() {
                        Some(association) => {
                            self.command_queue
                                .push_back(association.operate(mode, headers, handler));
                            Ok(true)
                        }
                        None => {
                            log::warn!(
                                "no association for command request with address: {}",
                                address
                            );
                            handler.complete(Err(TaskError::NoSuchAssociation(address).into()));
                            Ok(false)
                        }
                    }
                }
            },
            None => Err(Shutdown),
        }
    }

    async fn process_message_while_disconnected(&mut self) -> Result<(), Shutdown> {
        match self.user_queue.recv().await {
            Some(x) => match x {
                Message::Command(_, _, _, handler) => {
                    handler.complete(Err(CommandError::Task(TaskError::NoConnection)));
                    Ok(())
                }
            },
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
        let session = match self.associations.get_mut(source).ok() {
            Some(session) => session,
            None => {
                log::warn!(
                    "received unsolicited response from unknown address: {}",
                    source
                );
                return Ok(());
            }
        };

        session.process_iin(response.header.iin);

        if let Ok(objects) = response.objects {
            session.handle_response(response.header, objects);
        }

        if response.header.control.con {
            self.confirm_unsolicited(io, source, response.header.control.seq, writer)
                .await?;
        }

        Ok(())
    }

    async fn send_request<T, U>(
        &mut self,
        io: &mut T,
        address: u16,
        request: &U,
        writer: &mut WriterType,
    ) -> Result<Sequence, TaskError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
        U: RequestWriter,
    {
        // format the request
        let seq = self.associations.get_mut(address)?.increment_seq();
        let mut cursor = WriteCursor::new(&mut self.buffer);
        let mut hw = start_request(Control::request(seq), request.function(), &mut cursor)?;
        request.write(&mut hw)?;
        writer
            .write(self.level, io, address, cursor.written())
            .await?;
        Ok(seq)
    }

    pub(crate) async fn delay_for(&mut self, duration: Duration) -> Result<(), Shutdown> {
        let deadline = Instant::now().add(duration);

        loop {
            tokio::select! {
                result = self.process_message_while_disconnected() => {
                   result?;
                }
                _ = tokio::time::delay_until(tokio::time::Instant::from_std(deadline)) => {
                   return Ok(());
                }
            }
        }
    }

    fn get_next_task(&mut self) -> Next<Task> {
        if let Some(x) = self.command_queue.pop_front() {
            return Next::Now(x);
        }

        self.associations.next_task()
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
            let result = match self.get_next_task() {
                Next::Now(task) => self.run_task(io, task, writer, reader).await,
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
        task: Task,
        writer: &mut WriterType,
        reader: &mut ReaderType,
    ) -> Result<(), RunError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let result = match task.details {
            TaskType::Read(t) => {
                self.run_read_task(io, task.address, t, writer, reader)
                    .await
            }
            TaskType::NonRead(t) => {
                self.run_non_read_task(io, task.address, t, writer, reader)
                    .await
            }
        };

        // if a task error occurs, if might be a run error
        match result {
            Ok(()) => Ok(()),
            Err(err) => match err {
                TaskError::Shutdown => return Err(RunError::Shutdown),
                TaskError::Lower(err) => return Err(RunError::Link(err)),
                _ => Ok(()),
            },
        }
    }

    async fn run_non_read_task<T>(
        &mut self,
        io: &mut T,
        address: u16,
        mut task: NonReadTask,
        writer: &mut WriterType,
        reader: &mut ReaderType,
    ) -> Result<(), TaskError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        loop {
            let seq = match self.send_request(io, address, &task, writer).await {
                Ok(seq) => seq,
                Err(err) => {
                    task.on_task_error(err);
                    return Err(err);
                }
            };

            let deadline = self.timeout.from_now();

            loop {
                if let Err(err) = self.read_next_response(io, deadline, reader).await {
                    task.on_task_error(err);
                    return Err(err);
                }

                match self
                    .validate_non_read_response(address, seq, io, reader, writer)
                    .await
                {
                    // continue reading responses until timeout
                    Ok(None) => continue,
                    Ok(Some(response)) => {
                        match self.associations.get_mut(address) {
                            Err(x) => {
                                task.on_task_error(x.into());
                                return Err(x.into());
                            }
                            Ok(association) => {
                                association.process_iin(response.header.iin);
                                match task.handle(association, response) {
                                    NonReadTaskStatus::Complete => return Ok(()),
                                    NonReadTaskStatus::Next(next) => {
                                        task = next;
                                        // break from the inner loop and execute the next request
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(err) => {
                        task.on_task_error(err);
                        return Err(err);
                    }
                }
            }
        }
    }

    async fn validate_non_read_response<'a, T>(
        &mut self,
        address: u16,
        seq: Sequence,
        io: &mut T,
        reader: &'a mut ReaderType,
        writer: &mut WriterType,
    ) -> Result<Option<Response<'a>>, TaskError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let fragment = match reader.peek() {
            None => return Ok(None),
            Some(x) => x,
        };

        let parsed = match ParsedFragment::parse(self.level.receive(), fragment.data).ok() {
            None => return Ok(None),
            Some(x) => x,
        };

        let response = match parsed.to_response() {
            Err(err) => {
                log::warn!("{}", err);
                return Ok(None);
            }
            Ok(x) => x,
        };

        if response.header.unsolicited {
            self.handle_unsolicited(fragment.address.source, &response, io, writer)
                .await?;
            return Ok(None);
        }

        if fragment.address.source != address {
            log::warn!(
                "Received response from {} while expecting response from {}",
                fragment.address.source,
                address
            );
            return Ok(None);
        }

        if response.header.control.seq != seq {
            log::warn!(
                "unexpected sequence number is response: {}",
                response.header.control.seq.value()
            );
            return Ok(None);
        }

        if !response.header.control.is_fir_and_fin() {
            return Err(TaskError::MultiFragmentResponse);
        }

        Ok(Some(response))
    }

    async fn run_read_task<T>(
        &mut self,
        io: &mut T,
        address: u16,
        task: ReadTask,
        writer: &mut WriterType,
        reader: &mut ReaderType,
    ) -> Result<(), TaskError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let mut seq = self.send_request(io, address, &task, writer).await?;
        let mut is_first = true;

        // read responses until we get a FIN or an error occurs
        loop {
            let deadline = self.timeout.from_now();

            loop {
                self.read_next_response(io, deadline, reader).await?;
                match self
                    .process_read_response(address, is_first, seq, io, writer, reader)
                    .await?
                {
                    // continue reading responses on the inner loop
                    ReadResponseAction::Ignore => continue,
                    // read task complete
                    ReadResponseAction::Complete => {
                        task.complete(self.associations.get_mut(address)?);
                        return Ok(());
                    }
                    // break to the outer loop and read another response
                    ReadResponseAction::ReadNext => {
                        is_first = false;
                        seq = self.associations.get_mut(address)?.increment_seq();
                        break;
                    }
                }
            }
        }
    }

    async fn process_read_response<T>(
        &mut self,
        address: u16,
        is_first: bool,
        seq: Sequence,
        io: &mut T,
        writer: &mut WriterType,
        reader: &mut ReaderType,
    ) -> Result<ReadResponseAction, TaskError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let fragment = match reader.peek() {
            Some(x) => x,
            None => return Ok(ReadResponseAction::Ignore),
        };

        let parsed = match ParsedFragment::parse(self.level.receive(), fragment.data).ok() {
            Some(parsed) => parsed,
            None => return Ok(ReadResponseAction::Ignore),
        };

        let response = match parsed.to_response() {
            Ok(response) => response,
            Err(err) => {
                log::warn!("{}", err);
                return Ok(ReadResponseAction::Ignore);
            }
        };

        if response.header.unsolicited {
            self.handle_unsolicited(fragment.address.source, &response, io, writer)
                .await?;
            return Ok(ReadResponseAction::Ignore);
        }

        if fragment.address.source != address {
            log::warn!(
                "Received response from {} while expecting response from {}",
                fragment.address.source,
                address
            );
            return Ok(ReadResponseAction::Ignore);
        }

        if response.header.control.seq != seq {
            log::warn!(
                "response with seq: {} doesn't match expected seq: {}",
                response.header.control.seq.value(),
                seq.value()
            );
            return Ok(ReadResponseAction::Ignore);
        }

        // now do validations

        if response.header.control.fir && !is_first {
            return Err(TaskError::UnexpectedFir);
        }

        if !response.header.control.fir && is_first {
            return Err(TaskError::NeverReceivedFir);
        }

        if !response.header.control.fin && !response.header.control.con {
            return Err(TaskError::NonFinWithoutCon);
        }

        let association = self.associations.get_mut(address)?;
        association.process_iin(response.header.iin);
        // TODO - invoke task specific handler if present
        association.handle_response(response.header, response.objects?);

        if response.header.control.con {
            self.confirm_solicited(io, address, seq, writer).await?;
        }

        if response.header.control.fin {
            Ok(ReadResponseAction::Complete)
        } else {
            Ok(ReadResponseAction::ReadNext)
        }
    }

    async fn read_next_response<T>(
        &mut self,
        io: &mut T,
        deadline: tokio::time::Instant,
        reader: &mut ReaderType,
    ) -> Result<(), TaskError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        loop {
            tokio::select! {
                    _ = tokio::time::delay_until(deadline) => {
                         log::warn!("no response within timeout: {}", self.timeout);
                         return Err(TaskError::ResponseTimeout);
                    }
                    //
                    x = reader.read(io)  => {
                        return Ok(x?);
                    }
                    // unless shutdown, proceed to next event
                    y = self.process_message_while_connected() => {
                        y?;
                    }
            }
        }
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
        let map = AssociationMap::single(Association::new(
            1024,
            AssociationConfig::default(),
            NullHandler::boxed(),
        ));

        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let mut runner = Runner::new(
            ParseLogLevel::Nothing,
            Timeout::from_secs(1).unwrap(),
            map,
            rx,
        );

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
