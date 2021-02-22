use std::ops::Add;
use std::time::Duration;

use tracing::Instrument;

use crate::app::format::write;
use crate::app::format::write::start_request;
use crate::app::parse::parser::Response;
use crate::app::ControlField;
use crate::app::Sequence;
use crate::app::Shutdown;
use crate::app::Timeout;
use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::EndpointAddress;
use crate::link::LinkStatusResult;
use crate::master::association::{AssociationMap, Next};
use crate::master::error::TaskError;
use crate::master::messages::{MasterMsg, Message};
use crate::master::tasks::{AssociationTask, NonReadTask, ReadTask, RequestWriter, Task};
use crate::master::Association;
use crate::tokio::time::Instant;
use crate::transport::{TransportReader, TransportResponse, TransportWriter};
use crate::util::buffer::Buffer;
use crate::util::channel::Receiver;
use crate::util::phys::PhysLayer;

pub(crate) struct MasterSession {
    enabled: bool,
    decode_level: DecodeLevel,
    timeout: Timeout,
    associations: AssociationMap,
    messages: Receiver<Message>,
    tx_buffer: Buffer,
}

enum ReadResponseAction {
    Ignore,
    ReadNext,
    Complete,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StateChange {
    Disable,
    Shutdown,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum RunError {
    State(StateChange),
    Link(LinkError),
}

impl From<Shutdown> for StateChange {
    fn from(_: Shutdown) -> Self {
        StateChange::Shutdown
    }
}

impl From<StateChange> for RunError {
    fn from(x: StateChange) -> Self {
        RunError::State(x)
    }
}

impl From<LinkError> for RunError {
    fn from(err: LinkError) -> Self {
        RunError::Link(err)
    }
}

impl From<Shutdown> for RunError {
    fn from(_: Shutdown) -> Self {
        RunError::State(StateChange::Shutdown)
    }
}

impl MasterSession {
    pub(crate) const DEFAULT_TX_BUFFER_SIZE: usize = 2048;
    pub(crate) const MIN_TX_BUFFER_SIZE: usize = 249;

    pub(crate) const DEFAULT_RX_BUFFER_SIZE: usize = 2048;
    pub(crate) const MIN_RX_BUFFER_SIZE: usize = 2048;

    pub(crate) fn new(
        enabled: bool,
        decode_level: DecodeLevel,
        response_timeout: Timeout,
        tx_buffer_size: usize,
        messages: Receiver<Message>,
    ) -> Self {
        let tx_buffer_size = if tx_buffer_size < Self::MIN_TX_BUFFER_SIZE {
            tracing::warn!("Minimum TX buffer size is {}. Defaulting to this value because the provided value ({}) is too low.", Self::MIN_TX_BUFFER_SIZE, tx_buffer_size);
            Self::MIN_TX_BUFFER_SIZE
        } else {
            tx_buffer_size
        };

        Self {
            enabled,
            decode_level,
            timeout: response_timeout,
            associations: AssociationMap::new(),
            messages,
            tx_buffer: Buffer::new(tx_buffer_size),
        }
    }

    /// Wait for the defined duration, processing messages that are received in the meantime.
    pub(crate) async fn wait_for_retry(&mut self, duration: Duration) -> Result<(), StateChange> {
        let deadline = Instant::now().add(duration);

        loop {
            crate::tokio::select! {
                result = self.process_message(false) => {
                   result?;
                   if !self.enabled {
                       return Err(StateChange::Disable)
                   }
                }
                _ = crate::tokio::time::sleep_until(deadline) => {
                   return Ok(());
                }
            }
        }
    }

    /// wait until the session has been enabled
    pub(crate) async fn wait_for_enabled(&mut self) -> Result<(), Shutdown> {
        loop {
            if self.enabled {
                return Ok(());
            }

            if let Err(StateChange::Shutdown) = self.process_message(false).await {
                return Err(Shutdown);
            }
        }
    }

    /// Run the master until an error or shutdown occurs.
    pub(crate) async fn run(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> RunError {
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
    async fn idle_forever(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<(), RunError> {
        loop {
            let decode_level = self.decode_level;
            crate::tokio::select! {
                result = self.process_message(true) => {
                   // we need to recheck the tasks
                   return Ok(result?);
                }
                result = reader.read(io, decode_level) => {
                   result?;
                   match reader.pop_response() {
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
    async fn idle_until(
        &mut self,
        instant: Instant,
        io: &mut PhysLayer,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<(), RunError> {
        loop {
            let decode_level = self.decode_level;
            crate::tokio::select! {
                result = self.process_message(true) => {
                   // we need to recheck the tasks
                   return Ok(result?);
                }
                result = reader.read(io, decode_level) => {
                   result?;
                   match reader.pop_response() {
                        Some(TransportResponse::Response(source, response)) => {
                            self.notify_link_activity(source);
                            return self.handle_fragment_while_idle(io, writer, source, response).await
                        }
                        Some(TransportResponse::LinkLayerMessage(msg)) => self.notify_link_activity(msg.source),
                        None => return Ok(())
                   }
                }
                _ = crate::tokio::time::sleep_until(instant) => {
                   return Ok(());
                }
            }
        }
    }

    async fn process_message(&mut self, is_connected: bool) -> Result<(), StateChange> {
        let message = self.messages.receive().await?;
        match message {
            Message::Master(msg) => {
                self.process_master_message(msg);
                if is_connected && !self.enabled {
                    return Err(StateChange::Disable);
                }
            }
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

    fn process_master_message(&mut self, msg: MasterMsg) {
        match msg {
            MasterMsg::EnableCommunication(enable) => {
                if enable {
                    tracing::info!("communication enabled");
                } else {
                    tracing::info!("communication disabled");
                }
                self.enabled = enable;
            }
            MasterMsg::AddAssociation(address, config, handler, callback) => {
                callback.complete(
                    self.associations
                        .register(Association::new(address, config, handler)),
                );
            }
            MasterMsg::RemoveAssociation(address) => {
                self.associations.remove(address);
            }
            MasterMsg::SetDecodeLevel(level) => {
                self.decode_level = level;
            }
            MasterMsg::GetDecodeLevel(promise) => {
                promise.complete(Ok(self.decode_level));
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
    async fn run_task(
        &mut self,
        io: &mut PhysLayer,
        task: AssociationTask,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<(), RunError> {
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
                TaskError::State(x) => Err(RunError::State(x)),
                TaskError::Lower(err) => Err(RunError::Link(err)),
                _ => Ok(()),
            },
        }
    }

    async fn run_non_read_task(
        &mut self,
        io: &mut PhysLayer,
        destination: EndpointAddress,
        mut task: NonReadTask,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<(), TaskError> {
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
                    _ = crate::tokio::time::sleep_until(deadline) => {
                        tracing::warn!("no response within timeout: {}", self.timeout);
                        task.on_task_error(self.associations.get_mut(destination).ok(), TaskError::ResponseTimeout);
                        return Err(TaskError::ResponseTimeout);
                    }
                    x = reader.read(io, self.decode_level) => {
                        if let Err(err) = x {
                            task.on_task_error(self.associations.get_mut(destination).ok(), err.into());
                            return Err(err.into());
                        }

                        match reader.pop_response() {
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
    async fn validate_non_read_response<'a>(
        &mut self,
        destination: EndpointAddress,
        seq: Sequence,
        io: &mut PhysLayer,
        writer: &mut TransportWriter,
        source: EndpointAddress,
        response: Response<'a>,
    ) -> Result<Option<Response<'a>>, TaskError> {
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

    async fn run_read_task(
        &mut self,
        io: &mut PhysLayer,
        destination: EndpointAddress,
        task: ReadTask,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<(), TaskError> {
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

    async fn execute_read_task(
        &mut self,
        io: &mut PhysLayer,
        destination: EndpointAddress,
        task: &ReadTask,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<(), TaskError> {
        let mut seq = self.send_request(io, destination, task, writer).await?;
        let mut is_first = true;

        // read responses until we get a FIN or an error occurs
        loop {
            let deadline = self.timeout.deadline_from_now();

            loop {
                crate::tokio::select! {
                    _ = crate::tokio::time::sleep_until(deadline) => {
                            tracing::warn!("no response within timeout: {}", self.timeout);
                            return Err(TaskError::ResponseTimeout);
                    }
                    x = reader.read(io, self.decode_level) => {
                        x?;
                        match reader.pop_response() {
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
    async fn process_read_response(
        &mut self,
        destination: EndpointAddress,
        is_first: bool,
        seq: Sequence,
        task: &ReadTask,
        io: &mut PhysLayer,
        writer: &mut TransportWriter,
        source: EndpointAddress,
        response: Response<'_>,
    ) -> Result<ReadResponseAction, TaskError> {
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
    async fn handle_fragment_while_idle(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter,
        source: EndpointAddress,
        response: Response<'_>,
    ) -> Result<(), RunError> {
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

    async fn handle_unsolicited(
        &mut self,
        source: EndpointAddress,
        response: &Response<'_>,
        io: &mut PhysLayer,
        writer: &mut TransportWriter,
    ) -> Result<(), LinkError> {
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
    async fn confirm_solicited(
        &mut self,
        io: &mut PhysLayer,
        destination: EndpointAddress,
        seq: Sequence,
        writer: &mut TransportWriter,
    ) -> Result<(), LinkError> {
        let mut cursor = self.tx_buffer.write_cursor();
        write::confirm_solicited(seq, &mut cursor)?;
        writer
            .write(io, self.decode_level, destination.wrap(), cursor.written())
            .await?;
        Ok(())
    }

    async fn confirm_unsolicited(
        &mut self,
        io: &mut PhysLayer,
        destination: EndpointAddress,
        seq: Sequence,
        writer: &mut TransportWriter,
    ) -> Result<(), LinkError> {
        let mut cursor = self.tx_buffer.write_cursor();
        crate::app::format::write::confirm_unsolicited(seq, &mut cursor)?;

        writer
            .write(io, self.decode_level, destination.wrap(), cursor.written())
            .await?;
        Ok(())
    }

    async fn send_request<U>(
        &mut self,
        io: &mut PhysLayer,
        address: EndpointAddress,
        request: &U,
        writer: &mut TransportWriter,
    ) -> Result<Sequence, TaskError>
    where
        U: RequestWriter,
    {
        // format the request
        let association = self.associations.get_mut(address)?;
        let seq = association.increment_seq();
        let mut cursor = self.tx_buffer.write_cursor();
        let mut hw = start_request(ControlField::request(seq), request.function(), &mut cursor)?;
        request.write(&mut hw)?;
        writer
            .write(io, self.decode_level, address.wrap(), cursor.written())
            .await?;
        Ok(seq)
    }
}

// Link status stuff
impl MasterSession {
    async fn run_link_status_task(
        &mut self,
        io: &mut PhysLayer,
        destination: EndpointAddress,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<LinkStatusResult, TaskError> {
        // Send link status request
        tracing::info!("Sending link status request (for {})", destination);
        writer
            .write_link_status_request(io, self.decode_level, destination.wrap())
            .await?;

        loop {
            // Wait for something on the link
            crate::tokio::select! {
                _ = crate::tokio::time::sleep_until(self.timeout.deadline_from_now()) => {
                    tracing::warn!("no response within timeout: {}", self.timeout);
                    return Err(TaskError::ResponseTimeout);
                }
                x = reader.read(io, self.decode_level) => {
                    x?;
                    match reader.pop_response() {
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
