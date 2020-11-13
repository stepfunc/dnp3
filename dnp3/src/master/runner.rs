use crate::app::format::write;
use crate::app::parse::parser::{ParsedFragment, Response};
use crate::app::parse::DecodeLogLevel;
use crate::app::sequence::Sequence;
use crate::master::tasks::{AssociationTask, NonReadTask, ReadTask, RequestWriter, Task};
use crate::transport::{ReaderType, WriterType};

use crate::app::header::Control;
use crate::link::error::LinkError;
use crate::util::cursor::WriteCursor;

use crate::app::format::write::start_request;
use crate::app::timeout::Timeout;
use crate::master::association::{AssociationMap, Next};
use crate::master::messages::{MasterMsg, Message};

use crate::master::error::{Shutdown, TaskError};
use std::ops::Add;
use std::time::Duration;
use tokio::prelude::{AsyncRead, AsyncWrite};
use tokio::time::Instant;

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum RunError {
    Link(LinkError),
    Shutdown,
}

pub(crate) struct Runner {
    level: DecodeLogLevel,
    timeout: Timeout,
    associations: AssociationMap,
    user_queue: tokio::sync::mpsc::Receiver<Message>,
    buffer: [u8; 2048],
}

enum ReadResponseAction {
    Ignore,
    ReadNext,
    Complete,
}

impl Runner {
    pub(crate) fn new(
        level: DecodeLogLevel,
        response_timeout: Timeout,
        user_queue: tokio::sync::mpsc::Receiver<Message>,
    ) -> Self {
        Self {
            level,
            timeout: response_timeout,
            associations: AssociationMap::new(),
            user_queue,
            buffer: [0; 2048],
        }
    }

    /// Wait for the defined duration, processing messages that are received in the meantime.
    pub(crate) async fn delay_for(&mut self, duration: Duration) -> Result<(), Shutdown> {
        let deadline = Instant::now().add(duration);

        loop {
            tokio::select! {
                result = self.process_message(false) => {
                   result?;
                }
                _ = tokio::time::delay_until(deadline) => {
                   return Ok(());
                }
            }
        }
    }

    /// Run the master until an error or shutdown occurs.
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
                self.reset(err);
                writer.reset();
                reader.reset();
                return err;
            }
        }
    }

    /// Wait until a message is received or a response is received.
    ///
    /// Returns an error only if shutdown or link layer error occured.
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
                result = self.process_message(true) => {
                   // we need to recheck the tasks
                   return Ok(result?);
                }
                result = reader.read(io) => {
                   result?;
                   self.handle_fragment_while_idle(io, writer, reader).await?;
                }
            }
        }
    }

    /// Wait until a message is received, a response is received, or we reach the defined time.
    ///
    /// Returns an error only if shutdown or link layer error occured.
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
                result = self.process_message(true) => {
                   // we need to recheck the tasks
                   return Ok(result?);
                }
                result = reader.read(io) => {
                   result?;
                   self.handle_fragment_while_idle(io, writer, reader).await?;
                }
                _ = tokio::time::delay_until(instant) => {
                   return Ok(());
                }
            }
        }
    }

    async fn process_message(&mut self, is_connected: bool) -> Result<(), Shutdown> {
        match self.user_queue.recv().await {
            Some(msg) => {
                match msg {
                    Message::Master(msg) => self.process_master_message(msg),
                    Message::Association(msg) => {
                        if let Ok(association) = self.associations.get_mut(msg.address) {
                            association.process_message(msg.details, is_connected);
                        } else {
                            msg.on_association_failure();
                        }
                    }
                }
                Ok(())
            }
            None => Err(Shutdown),
        }
    }

    fn process_master_message(&mut self, msg: MasterMsg) {
        match msg {
            MasterMsg::AddAssociation(association, callback) => {
                callback.complete(self.associations.register(association));
            }
            MasterMsg::RemoveAssociation(address) => {
                self.associations.remove(address);
            }
            MasterMsg::SetDecodeLogLevel(level) => {
                self.level = level;
            }
            MasterMsg::GetDecodeLogLevel(promise) => {
                promise.complete(Ok(self.level));
            }
        }
    }

    async fn read_next_response<T>(
        &mut self,
        io: &mut T,
        deadline: Instant,
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
                    x = reader.read(io)  => {
                        return Ok(x?);
                    }
                    y = self.process_message(true) => {
                        y?; // unless shutdown, proceed to next event
                    }
            }
        }
    }

    fn reset(&mut self, err: RunError) {
        self.associations.reset(err);
    }
}

// Task processing
impl Runner {
    /// Run a specific task.
    ///
    /// Returns an error only if shutdown or link layer error occured.
    async fn run_task<T>(
        &mut self,
        io: &mut T,
        task: AssociationTask,
        writer: &mut WriterType,
        reader: &mut ReaderType,
    ) -> Result<(), RunError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let result = match task.details {
            Task::Read(t) => {
                self.run_read_task(io, task.address, t, writer, reader)
                    .await
            }
            Task::NonRead(t) => {
                self.run_non_read_task(io, task.address, t, writer, reader)
                    .await
            }
        };

        // if a task error occurs, if might be a run error
        match result {
            Ok(()) => Ok(()),
            Err(err) => match err {
                TaskError::Shutdown => Err(RunError::Shutdown),
                TaskError::Lower(err) => Err(RunError::Link(err)),
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
                    task.on_task_error(self.associations.get_mut(address).ok(), err);
                    return Err(err);
                }
            };

            let request_tx = std::time::SystemTime::now();

            let deadline = self.timeout.deadline_from_now();

            loop {
                if let Err(err) = self.read_next_response(io, deadline, reader).await {
                    task.on_task_error(self.associations.get_mut(address).ok(), err);
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
                                task.on_task_error(None, x.into());
                                return Err(x.into());
                            }
                            Ok(association) => {
                                association.process_iin(response.header.iin);
                                match task.handle(request_tx, association, response) {
                                    None => return Ok(()),
                                    Some(next) => {
                                        task = next;
                                        // break from the inner loop and execute the next request
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(err) => {
                        task.on_task_error(self.associations.get_mut(address).ok(), err);
                        return Err(err);
                    }
                }
            }
        }
    }

    // As of 1.43, clippy erroneously says these lifetimes aren't required
    #[allow(clippy::needless_lifetimes)]
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

        if response.header.function.is_unsolicited() {
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
        let result = self
            .execute_read_task(io, address, &task, writer, reader)
            .await;

        let association = self.associations.get_mut(address).ok();

        match result {
            Ok(_) => {
                if let Some(association) = association {
                    task.complete(association);
                } else {
                    task.on_task_error(None, TaskError::NoSuchAssociation(address));
                }
            }
            Err(err) => task.on_task_error(association, err),
        }

        result
    }

    async fn execute_read_task<T>(
        &mut self,
        io: &mut T,
        address: u16,
        task: &ReadTask,
        writer: &mut WriterType,
        reader: &mut ReaderType,
    ) -> Result<(), TaskError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let mut seq = self.send_request(io, address, task, writer).await?;
        let mut is_first = true;

        // read responses until we get a FIN or an error occurs
        loop {
            let deadline = self.timeout.deadline_from_now();

            loop {
                self.read_next_response(io, deadline, reader).await?;

                match self
                    .process_read_response(address, is_first, seq, &task, io, writer, reader)
                    .await?
                {
                    // continue reading responses on the inner loop
                    ReadResponseAction::Ignore => continue,
                    // read task complete
                    ReadResponseAction::Complete => {
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

    #[allow(clippy::too_many_arguments)] // TODO
    async fn process_read_response<T>(
        &mut self,
        address: u16,
        is_first: bool,
        seq: Sequence,
        task: &ReadTask,
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

        if response.header.function.is_unsolicited() {
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
        task.process_response(association, response.header, response.objects?);

        if response.header.control.con {
            self.confirm_solicited(io, address, seq, writer).await?;
        }

        if response.header.control.fin {
            Ok(ReadResponseAction::Complete)
        } else {
            Ok(ReadResponseAction::ReadNext)
        }
    }

    fn get_next_task(&mut self) -> Next<AssociationTask> {
        self.associations.next_task()
    }
}

// Unsolicited processing
impl Runner {
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

        if response.header.function.is_unsolicited() {
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
        let association = match self.associations.get_mut(source).ok() {
            Some(x) => x,
            None => {
                log::warn!(
                    "received unsolicited response from unknown address: {}",
                    source
                );
                return Ok(());
            }
        };

        association.process_iin(response.header.iin);

        if let Ok(objects) = response.objects {
            association.handle_unsolicited_response(response.header, objects);
        }

        if response.header.control.con {
            self.confirm_unsolicited(io, source, response.header.control.seq, writer)
                .await?;
        }

        Ok(())
    }
}

// Sending methods
impl Runner {
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
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::master::association::Configuration;
    use crate::master::handle::{MasterHandle, NullHandler};
    use crate::transport::create_transport_layer;
    use tokio_test::io::Builder;

    #[tokio::test]
    async fn performs_startup_sequence_with_device_restart_asserted() {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let mut runner = Runner::new(DecodeLogLevel::Nothing, Timeout::from_secs(1).unwrap(), rx);
        let mut master = MasterHandle::new(tx);

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

        let (mut reader, mut writer) = create_transport_layer(true, 1);

        let mut master_task =
            tokio_test::task::spawn(runner.run(&mut io, &mut writer, &mut reader));
        let association = {
            let mut add_task = tokio_test::task::spawn(master.add_association(
                1024,
                Configuration::default(),
                NullHandler::boxed(),
            ));
            tokio_test::assert_pending!(add_task.poll());
            tokio_test::assert_pending!(master_task.poll());
            tokio_test::assert_ready_ok!(add_task.poll())
        };

        tokio_test::assert_pending!(master_task.poll());

        // causes the task to shutdown
        drop(master);
        drop(association);

        tokio_test::assert_ready_eq!(master_task.poll(), RunError::Shutdown);
        drop(master_task);
        assert_eq!(writer.num_writes(), 4);
        assert_eq!(reader.num_reads(), 4);
    }
}
