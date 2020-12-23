use crate::app::format::write;
use crate::app::format::write::start_request;
use crate::app::header::Control;
use crate::app::parse::parser::Response;
use crate::app::parse::DecodeLogLevel;
use crate::app::sequence::Sequence;
use crate::app::timeout::Timeout;
use crate::app::types::LinkStatusResult;
use crate::entry::EndpointAddress;
use crate::link::error::LinkError;
use crate::master::association::{AssociationMap, Next};
use crate::master::error::TaskError;
use crate::master::messages::{MasterMsg, Message};
use crate::master::tasks::{AssociationTask, NonReadTask, ReadTask, RequestWriter, Task};
use crate::tokio::io::{AsyncRead, AsyncWrite};
use crate::tokio::time::Instant;
use crate::transport::{TransportReader, TransportResponse, TransportWriter};
use crate::util::buffer::Buffer;
use crate::util::io::IOStream;
use crate::util::task::{RunError, Shutdown};
use std::ops::Add;
use std::time::Duration;
use tracing::Instrument;

pub(crate) struct MasterSession {
    level: DecodeLogLevel,
    timeout: Timeout,
    associations: AssociationMap,
    user_queue: crate::tokio::sync::mpsc::Receiver<Message>,
    tx_buffer: Buffer,
}

enum ReadResponseAction {
    Ignore,
    ReadNext,
    Complete,
}

impl MasterSession {
    pub(crate) const DEFAULT_TX_BUFFER_SIZE: usize = 2048;
    pub(crate) const MIN_TX_BUFFER_SIZE: usize = 249;

    pub(crate) const DEFAULT_RX_BUFFER_SIZE: usize = 2048;
    pub(crate) const MIN_RX_BUFFER_SIZE: usize = 2048;

    pub(crate) fn new(
        level: DecodeLogLevel,
        response_timeout: Timeout,
        tx_buffer_size: usize,
        user_queue: crate::tokio::sync::mpsc::Receiver<Message>,
    ) -> Self {
        let tx_buffer_size = if tx_buffer_size < Self::MIN_TX_BUFFER_SIZE {
            tracing::warn!("Minimum TX buffer size is {}. Defaulting to this value because the provided value ({}) is too low.", Self::MIN_TX_BUFFER_SIZE, tx_buffer_size);
            Self::MIN_TX_BUFFER_SIZE
        } else {
            tx_buffer_size
        };

        Self {
            level,
            timeout: response_timeout,
            associations: AssociationMap::new(),
            user_queue,
            tx_buffer: Buffer::new(tx_buffer_size),
        }
    }

    /// Wait for the defined duration, processing messages that are received in the meantime.
    pub(crate) async fn delay_for(&mut self, duration: Duration) -> Result<(), Shutdown> {
        let deadline = Instant::now().add(duration);

        loop {
            crate::tokio::select! {
                result = self.process_message(false) => {
                   result?;
                }
                _ = crate::tokio::time::delay_until(deadline) => {
                   return Ok(());
                }
            }
        }
    }

    /// Run the master until an error or shutdown occurs.
    pub(crate) async fn run<T>(
        &mut self,
        io: &mut T,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> RunError
    where
        T: IOStream,
    {
        loop {
            let result = match self.get_next_task() {
                Next::Now(task) => {
                    let id = task.details.get_id();
                    let address = task.address.raw_value();
                    self.run_task(io, task, writer, reader)
                        .instrument(tracing::info_span!("Task", "type" = ?id, "dest" = address))
                        .await
                }
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
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<(), RunError>
    where
        T: IOStream,
    {
        loop {
            crate::tokio::select! {
                result = self.process_message(true) => {
                   // we need to recheck the tasks
                   return Ok(result?);
                }
                result = reader.read(io) => {
                   result?;
                   match reader.pop_response(self.level) {
                        Some(TransportResponse::Response(source, response)) => {
                            self.notify_link_activity(source);
                            return self.handle_fragment_while_idle(io, writer, source, response).await
                        }
                        Some(TransportResponse::LinkLayerMessage(msg)) => self.notify_link_activity(msg.source),
                        None => return Ok(())
                   }
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
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<(), RunError>
    where
        T: IOStream,
    {
        loop {
            crate::tokio::select! {
                result = self.process_message(true) => {
                   // we need to recheck the tasks
                   return Ok(result?);
                }
                result = reader.read(io) => {
                   result?;
                   match reader.pop_response(self.level) {
                        Some(TransportResponse::Response(source, response)) => {
                            self.notify_link_activity(source);
                            return self.handle_fragment_while_idle(io, writer, source, response).await
                        }
                        Some(TransportResponse::LinkLayerMessage(msg)) => self.notify_link_activity(msg.source),
                        None => return Ok(())
                   }
                }
                _ = crate::tokio::time::delay_until(instant) => {
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

    fn reset(&mut self, err: RunError) {
        self.associations.reset(err);
    }
}

// Task processing
impl MasterSession {
    /// Run a specific task.
    ///
    /// Returns an error only if shutdown or link layer error occured.
    async fn run_task<T>(
        &mut self,
        io: &mut T,
        task: AssociationTask,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<(), RunError>
    where
        T: IOStream,
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
            Task::LinkStatus(promise) => {
                match self
                    .run_link_status_task(io, task.address, writer, reader)
                    .await
                {
                    Ok(result) => {
                        promise.complete(Ok(result));
                        Ok(())
                    }
                    Err(err) => {
                        promise.complete(Err(err));
                        Err(err)
                    }
                }
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
        destination: EndpointAddress,
        mut task: NonReadTask,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<(), TaskError>
    where
        T: IOStream,
    {
        loop {
            let seq = match self.send_request(io, destination, &task, writer).await {
                Ok(seq) => seq,
                Err(err) => {
                    task.on_task_error(self.associations.get_mut(destination).ok(), err);
                    return Err(err);
                }
            };

            let deadline = self.timeout.deadline_from_now();

            loop {
                crate::tokio::select! {
                    _ = crate::tokio::time::delay_until(deadline) => {
                        tracing::warn!("no response within timeout: {}", self.timeout);
                        task.on_task_error(self.associations.get_mut(destination).ok(), TaskError::ResponseTimeout);
                        return Err(TaskError::ResponseTimeout);
                    }
                    x = reader.read(io) => {
                        if let Err(err) = x {
                            task.on_task_error(self.associations.get_mut(destination).ok(), err.into());
                            return Err(err.into());
                        }

                        match reader.pop_response(self.level) {
                            Some(TransportResponse::Response(source, response)) => {
                                self.notify_link_activity(source);

                                let result = self
                                    .validate_non_read_response(destination, seq, io, writer, source, response)
                                    .await;

                                match result {
                                    // continue reading responses until timeout
                                    Ok(None) => continue,
                                    Ok(Some(response)) => {
                                        match self.associations.get_mut(destination) {
                                            Err(x) => {
                                                task.on_task_error(None, x.into());
                                                return Err(x.into());
                                            }
                                            Ok(association) => {
                                                association.process_iin(response.header.iin);
                                                match task.handle(association, response) {
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
                                        task.on_task_error(self.associations.get_mut(destination).ok(), err);
                                        return Err(err);
                                    }
                                }
                            }
                            Some(TransportResponse::LinkLayerMessage(msg)) => self.notify_link_activity(msg.source),
                            None => continue
                        }
                    }
                    y = self.process_message(true) => {
                        match y {
                            Ok(_) => (), // unless shutdown, proceed to next event
                            Err(err) => {
                                task.on_task_error(self.associations.get_mut(destination).ok(), err.into());
                                return Err(err.into());
                            }
                        }
                    }
                }
            }
        }
    }

    #[allow(clippy::needless_lifetimes)]
    async fn validate_non_read_response<'a, T>(
        &mut self,
        destination: EndpointAddress,
        seq: Sequence,
        io: &mut T,
        writer: &mut TransportWriter,
        source: EndpointAddress,
        response: Response<'a>,
    ) -> Result<Option<Response<'a>>, TaskError>
    where
        T: IOStream,
    {
        if response.header.function.is_unsolicited() {
            self.handle_unsolicited(source, &response, io, writer)
                .await?;
            return Ok(None);
        }

        if source != destination {
            tracing::warn!(
                "Received response from {} while expecting response from {}",
                source,
                destination
            );
            return Ok(None);
        }

        if response.header.control.seq != seq {
            tracing::warn!(
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
        destination: EndpointAddress,
        task: ReadTask,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<(), TaskError>
    where
        T: IOStream,
    {
        let result = self
            .execute_read_task(io, destination, &task, writer, reader)
            .await;

        let association = self.associations.get_mut(destination).ok();

        match result {
            Ok(_) => {
                if let Some(association) = association {
                    task.complete(association);
                } else {
                    task.on_task_error(None, TaskError::NoSuchAssociation(destination));
                }
            }
            Err(err) => task.on_task_error(association, err),
        }

        result
    }

    async fn execute_read_task<T>(
        &mut self,
        io: &mut T,
        destination: EndpointAddress,
        task: &ReadTask,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<(), TaskError>
    where
        T: IOStream,
    {
        let mut seq = self.send_request(io, destination, task, writer).await?;
        let mut is_first = true;

        // read responses until we get a FIN or an error occurs
        loop {
            let deadline = self.timeout.deadline_from_now();

            loop {
                crate::tokio::select! {
                    _ = crate::tokio::time::delay_until(deadline) => {
                            tracing::warn!("no response within timeout: {}", self.timeout);
                            return Err(TaskError::ResponseTimeout);
                    }
                    x = reader.read(io) => {
                        x?;
                        match reader.pop_response(self.level) {
                            Some(TransportResponse::Response(source, response)) => {
                                self.notify_link_activity(source);
                                let action = self.process_read_response(destination, is_first, seq, &task, io, writer, source, response).await?;
                                match action {
                                    // continue reading responses on the inner loop
                                    ReadResponseAction::Ignore => continue,
                                    // read task complete
                                    ReadResponseAction::Complete => return Ok(()),
                                    // break to the outer loop and read another response
                                    ReadResponseAction::ReadNext => {
                                        is_first = false;
                                        seq = self.associations.get_mut(destination)?.increment_seq();
                                        break;
                                    }
                                }
                            }
                            Some(TransportResponse::LinkLayerMessage(msg)) => self.notify_link_activity(msg.source),
                            None => continue
                        }
                    }
                    y = self.process_message(true) => {
                        y?; // unless shutdown, proceed to next event
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)] // TODO
    async fn process_read_response<T>(
        &mut self,
        destination: EndpointAddress,
        is_first: bool,
        seq: Sequence,
        task: &ReadTask,
        io: &mut T,
        writer: &mut TransportWriter,
        source: EndpointAddress,
        response: Response<'_>,
    ) -> Result<ReadResponseAction, TaskError>
    where
        T: IOStream,
    {
        if response.header.function.is_unsolicited() {
            self.handle_unsolicited(source, &response, io, writer)
                .await?;
            return Ok(ReadResponseAction::Ignore);
        }

        if source != destination {
            tracing::warn!(
                "Received response from {} while expecting response from {}",
                source,
                destination
            );
            return Ok(ReadResponseAction::Ignore);
        }

        if response.header.control.seq != seq {
            tracing::warn!(
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

        let association = self.associations.get_mut(destination)?;
        association.process_iin(response.header.iin);
        task.process_response(association, response.header, response.objects?);

        if response.header.control.con {
            self.confirm_solicited(io, destination, seq, writer).await?;
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
impl MasterSession {
    async fn handle_fragment_while_idle<T>(
        &mut self,
        io: &mut T,
        writer: &mut TransportWriter,
        source: EndpointAddress,
        response: Response<'_>,
    ) -> Result<(), RunError>
    where
        T: IOStream,
    {
        if response.header.function.is_unsolicited() {
            self.handle_unsolicited(source, &response, io, writer)
                .await?;
        } else {
            tracing::warn!(
                "unexpected response with sequence: {}",
                response.header.control.seq.value()
            )
        }

        Ok(())
    }

    async fn handle_unsolicited<T>(
        &mut self,
        source: EndpointAddress,
        response: &Response<'_>,
        io: &mut T,
        writer: &mut TransportWriter,
    ) -> Result<(), LinkError>
    where
        T: IOStream,
    {
        let association = match self.associations.get_mut(source).ok() {
            Some(x) => x,
            None => {
                tracing::warn!(
                    "received unsolicited response from unknown address: {}",
                    source
                );
                return Ok(());
            }
        };

        association.process_iin(response.header.iin);

        let valid = association.handle_unsolicited_response(response);

        // Send confirmation if required and wasn't ignored
        if valid && response.header.control.con {
            self.confirm_unsolicited(io, source, response.header.control.seq, writer)
                .await?;
        }

        Ok(())
    }
}

// Sending methods
impl MasterSession {
    async fn confirm_solicited<T>(
        &mut self,
        io: &mut T,
        destination: EndpointAddress,
        seq: Sequence,
        writer: &mut TransportWriter,
    ) -> Result<(), LinkError>
    where
        T: IOStream,
    {
        let mut cursor = self.tx_buffer.write_cursor();
        write::confirm_solicited(seq, &mut cursor)?;
        writer
            .write(io, self.level, destination.wrap(), cursor.written())
            .await?;
        Ok(())
    }

    async fn confirm_unsolicited<T>(
        &mut self,
        io: &mut T,
        destination: EndpointAddress,
        seq: Sequence,
        writer: &mut TransportWriter,
    ) -> Result<(), LinkError>
    where
        T: IOStream,
    {
        let mut cursor = self.tx_buffer.write_cursor();
        crate::app::format::write::confirm_unsolicited(seq, &mut cursor)?;

        writer
            .write(io, self.level, destination.wrap(), cursor.written())
            .await?;
        Ok(())
    }

    async fn send_request<T, U>(
        &mut self,
        io: &mut T,
        address: EndpointAddress,
        request: &U,
        writer: &mut TransportWriter,
    ) -> Result<Sequence, TaskError>
    where
        T: IOStream,
        U: RequestWriter,
    {
        // format the request
        let association = self.associations.get_mut(address)?;
        let seq = association.increment_seq();
        let mut cursor = self.tx_buffer.write_cursor();
        let mut hw = start_request(Control::request(seq), request.function(), &mut cursor)?;
        request.write(&mut hw)?;
        writer
            .write(io, self.level, address.wrap(), cursor.written())
            .await?;
        Ok(seq)
    }
}

// Link status stuff
impl MasterSession {
    async fn run_link_status_task<T>(
        &mut self,
        io: &mut T,
        destination: EndpointAddress,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<LinkStatusResult, TaskError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        // Send link status request
        tracing::info!("Sending link status request (for {})", destination);
        writer
            .write_link_status_request(io, destination.wrap())
            .await?;

        loop {
            // Wait for something on the link
            crate::tokio::select! {
                _ = crate::tokio::time::delay_until(self.timeout.deadline_from_now()) => {
                    tracing::warn!("no response within timeout: {}", self.timeout);
                    return Err(TaskError::ResponseTimeout);
                }
                x = reader.read(io) => {
                    x?;
                    match reader.pop_response(self.level) {
                        Some(TransportResponse::Response(source, response)) => {
                            self.notify_link_activity(source);
                            self.handle_fragment_while_idle(io, writer, source, response).await?;
                            return Ok(LinkStatusResult::UnexpectedResponse);
                        }
                        Some(TransportResponse::LinkLayerMessage(msg)) => {
                            self.notify_link_activity(msg.source);
                            return Ok(LinkStatusResult::Success);
                        }
                        None => continue
                    }
                }
                y = self.process_message(true) => {
                    y?; // unless shutdown, proceed to next event
                }
            }
        }
    }

    fn notify_link_activity(&mut self, source: EndpointAddress) {
        if let Ok(association) = self.associations.get_mut(source) {
            association.on_link_activity();
        }
    }
}
