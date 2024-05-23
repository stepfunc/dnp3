use tracing::Instrument;

use crate::app::format::write;
use crate::app::parse::parser::Response;
use crate::app::{BufferSize, ControlField, Sequence};
use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::EndpointAddress;
use crate::master::association::{AssociationMap, Next};
use crate::master::error::TaskError;
use crate::master::messages::{MasterMsg, Message};
use crate::master::tasks::{AppTask, AssociationTask, NonReadTask, ReadTask, RequestWriter, Task};
use crate::master::{Association, MasterChannelConfig};
use crate::transport::{FragmentAddr, TransportReader, TransportResponse, TransportWriter};
use crate::util::buffer::Buffer;
use crate::util::channel::Receiver;
use crate::util::phys::PhysLayer;

use crate::link::reader::LinkModes;
use crate::util::session::{Enabled, RunError, StopReason};
use tokio::time::Instant;

/// combines the session with transport stuff
pub(crate) struct MasterTask {
    session: MasterSession,
    reader: TransportReader,
    writer: TransportWriter,
}

impl MasterTask {
    pub(crate) fn new(
        initial_state: Enabled,
        link_modes: LinkModes,
        config: MasterChannelConfig,
        messages: Receiver<Message>,
    ) -> Self {
        let session = MasterSession::new(
            initial_state,
            config.decode_level,
            config.tx_buffer_size,
            messages,
        );
        let (reader, writer) = crate::transport::create_master_transport_layer(
            link_modes,
            config.master_address,
            config.rx_buffer_size,
        );
        Self {
            session,
            reader,
            writer,
        }
    }

    pub(crate) fn seed_link(&mut self, seed_data: &[u8]) -> Result<(), scursor::WriteError> {
        self.reader.seed_link(seed_data)
    }

    #[cfg(test)]
    pub(crate) fn set_rx_frame_info(&mut self, info: crate::link::header::FrameInfo) {
        self.reader.get_inner().set_rx_frame_info(info);
    }

    pub(crate) fn enabled(&self) -> Enabled {
        self.session.enabled
    }

    pub(crate) async fn run(&mut self, io: &mut PhysLayer) -> RunError {
        let ret = self
            .session
            .run(io, &mut self.writer, &mut self.reader)
            .await;

        self.writer.reset();
        self.reader.reset();

        ret
    }

    pub(crate) async fn process_next_message(&mut self) -> Result<(), StopReason> {
        self.session.process_next_message().await
    }
}

struct MasterSession {
    enabled: Enabled,
    decode_level: DecodeLevel,
    associations: AssociationMap,
    messages: Receiver<Message>,
    tx_buffer: Buffer,
}

enum ReadResponseAction {
    Ignore,
    ReadNext,
    Complete,
}

impl MasterSession {
    fn new(
        initial_state: Enabled,
        decode_level: DecodeLevel,
        tx_buffer_size: BufferSize<249, 2048>,
        messages: Receiver<Message>,
    ) -> Self {
        Self {
            enabled: initial_state,
            decode_level,
            associations: AssociationMap::new(),
            messages,
            tx_buffer: tx_buffer_size.create_buffer(),
        }
    }

    /// Run the master until an error or shutdown occurs.
    async fn run(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> RunError {
        loop {
            let result = match self.get_next_task() {
                Next::Now(task) => {
                    let id = task.details.get_id();
                    let address = task.dest.link.raw_value();
                    self.run_task(io, task, writer, reader)
                        .instrument(tracing::info_span!("task", "type" = ?id, "dest" = address))
                        .await
                }
                Next::NotBefore(time) => self.idle_until(time, io, writer, reader).await,
                Next::None => self.idle_forever(io, writer, reader).await,
            };

            if let Err(err) = result {
                self.reset(err);

                if RunError::Stop(StopReason::Shutdown) == err {
                    self.messages.close_and_drain();
                }

                return err;
            }
        }
    }

    /// Wait until a message is received or a response is received.
    ///
    /// Returns an error only if shutdown or link layer error occurred.
    async fn idle_forever(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<(), RunError> {
        loop {
            let decode_level = self.decode_level;
            tokio::select! {
                result = self.process_message(true) => {
                   // we need to recheck the tasks
                   return Ok(result?);
                }
                result = reader.read(io, decode_level) => {
                   result?;
                   match reader.pop_response() {
                        Some(TransportResponse::Response(addr, response)) => {
                            self.notify_link_activity(addr.link);
                            return self.handle_fragment_while_idle(io, writer, addr, response).await
                        }
                        Some(TransportResponse::LinkLayerMessage(msg)) => self.notify_link_activity(msg.source),
                        Some(TransportResponse::Error(_)) => return Ok(()), // ignore the malformed response
                        None => return Ok(()),
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
            tokio::select! {
                result = self.process_message(true) => {
                   // we need to recheck the tasks
                   return Ok(result?);
                }
                result = reader.read(io, decode_level) => {
                   result?;
                   match reader.pop_response() {
                        Some(TransportResponse::Response(addr, response)) => {
                            self.notify_link_activity(addr.link);
                            return self.handle_fragment_while_idle(io, writer, addr, response).await
                        }
                        Some(TransportResponse::LinkLayerMessage(msg)) => self.notify_link_activity(msg.source),
                        Some(TransportResponse::Error(_)) => return Ok(()), // ignore the malformed response
                        None => return Ok(()),
                   }
                }
                _ = tokio::time::sleep_until(instant) => {
                   return Ok(());
                }
            }
        }
    }

    pub(crate) async fn process_next_message(&mut self) -> Result<(), StopReason> {
        self.process_message(false).await
    }

    async fn process_message(&mut self, is_connected: bool) -> Result<(), StopReason> {
        let message = self.messages.receive().await?;
        match message {
            Message::Master(msg) => {
                self.process_master_message(msg);
                if is_connected && self.enabled != Enabled::Yes {
                    return Err(StopReason::Disable);
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
            MasterMsg::EnableCommunication(enabled) => {
                match enabled {
                    Enabled::Yes => tracing::info!("communication enabled"),
                    Enabled::No => tracing::info!("communication disabled"),
                }
                self.enabled = enabled;
            }
            MasterMsg::AddAssociation(
                address,
                config,
                read_handler,
                assoc_handler,
                assoc_info,
                callback,
            ) => {
                callback.complete(self.associations.register(Association::new(
                    address,
                    config,
                    read_handler,
                    assoc_handler,
                    assoc_info,
                )));
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

    /// Run a specific task.
    ///
    /// Returns an error only if shutdown or link layer error occurred.
    async fn run_task(
        &mut self,
        io: &mut PhysLayer,
        task: AssociationTask,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<(), RunError> {
        let result = match task.details {
            Task::App(t) => {
                let task_type = t.as_task_type();
                let function = t.function();

                if let Ok(assoc) = self.associations.get_mut(task.dest.link) {
                    assoc.notify_task_start(task_type, function, assoc.seq());
                }

                let res: Result<Sequence, TaskError> = match t {
                    AppTask::Read(t) => self.run_read_task(io, task.dest, t, writer, reader).await,
                    AppTask::NonRead(t) => {
                        self.run_non_read_task(io, task.dest, t, writer, reader)
                            .await
                    }
                };

                if let Ok(assoc) = self.associations.get_mut(task.dest.link) {
                    match res {
                        Ok(seq) => {
                            assoc.notify_task_success(task_type, function, seq);
                        }
                        Err(err) => {
                            assoc.notify_task_fail(task_type, err);
                        }
                    }
                }

                res.map(|_| ())
            }
            Task::LinkStatus(promise) => {
                let res = self
                    .run_link_status_task(io, task.dest, writer, reader)
                    .await;
                promise.complete(res);
                res
            }
        };

        // if a task error occurs, if might be a run error
        match result {
            Ok(()) => Ok(()),
            Err(err) => match err {
                TaskError::Shutdown => Err(RunError::Stop(StopReason::Shutdown)),
                TaskError::Disabled => Err(RunError::Stop(StopReason::Disable)),
                TaskError::Link(err) => Err(RunError::Link(err)),
                _ => Ok(()),
            },
        }
    }

    async fn run_non_read_task(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        task: NonReadTask,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<Sequence, TaskError> {
        let mut next = NextStep::Continue(task);

        loop {
            next = match next {
                NextStep::Continue(task) => {
                    self.run_single_non_read_task(io, dest, task, writer, reader)
                        .await?
                }
                NextStep::Complete(seq) => {
                    return Ok(seq);
                }
            }
        }
    }

    async fn run_single_non_read_task(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        task: NonReadTask,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<NextStep, TaskError> {
        let seq = match self.send_request(io, dest, &task, writer).await {
            Ok(seq) => seq,
            Err(err) => {
                task.on_task_error(self.associations.get_mut(dest.link).ok(), err);
                return Err(err);
            }
        };

        let timeout = self.associations.get_timeout(dest.link)?;
        let deadline = timeout.deadline_from_now();

        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => {
                    tracing::warn!("no response within timeout: {}", timeout);
                    task.on_task_error(self.associations.get_mut(dest.link).ok(), TaskError::ResponseTimeout);
                    return Err(TaskError::ResponseTimeout);
                }
                x = reader.read(io, self.decode_level) => {
                    if let Err(err) = x {
                        task.on_task_error(self.associations.get_mut(dest.link).ok(), err.into());
                        return Err(err.into());
                    }

                    match reader.pop_response() {
                        Some(TransportResponse::Response(source, response)) => {
                            self.notify_link_activity(dest.link);

                            let result = self
                                .validate_non_read_response(dest, seq, io, writer, source, response)
                                .await;

                            match result {
                                // continue reading responses until timeout
                                Ok(None) => continue,
                                Ok(Some(response)) => {
                                    match self.associations.get_mut(dest.link) {
                                        Err(x) => {
                                            task.on_task_error(None, x.into());
                                            return Err(x.into());
                                        }
                                        Ok(association) => {
                                            association.process_iin(response.header.iin);
                                            return match task.handle_response(association, response).await? {
                                                Some(next) => {
                                                    Ok(NextStep::Continue(next))
                                                }
                                                None => {
                                                    Ok(NextStep::Complete(seq))
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(err) => {
                                    task.on_task_error(self.associations.get_mut(dest.link).ok(), err);
                                    return Err(err);
                                }
                            }
                        }
                        Some(TransportResponse::LinkLayerMessage(msg)) => self.notify_link_activity(msg.source),
                        Some(TransportResponse::Error(err)) => {
                            task.on_task_error(self.associations.get_mut(dest.link).ok(), err.into());
                            return Err(err.into());
                        },
                        None => continue,
                    }
                }
                y = self.process_message(true) => {
                    match y {
                        Ok(_) => (), // unless shutdown, proceed to next event
                        Err(err) => {
                            task.on_task_error(self.associations.get_mut(dest.link).ok(), err.into());
                            return Err(err.into());
                        }
                    }
                }
            }
        }
    }

    async fn validate_non_read_response<'a>(
        &mut self,
        destination: FragmentAddr,
        seq: Sequence,
        io: &mut PhysLayer,
        writer: &mut TransportWriter,
        source: FragmentAddr,
        response: Response<'a>,
    ) -> Result<Option<Response<'a>>, TaskError> {
        if response.header.function.is_unsolicited() {
            self.handle_unsolicited(source, &response, io, writer)
                .await?;
            return Ok(None);
        }

        if source.link != destination.link {
            tracing::warn!(
                "Received response from {} while expecting response from {}",
                source.link,
                destination.link
            );
            return Ok(None);
        }

        if response.header.control.seq != seq {
            tracing::warn!(
                "unexpected sequence number in response: {}",
                response.header.control.seq.value()
            );
            return Ok(None);
        }

        if !response.header.control.is_fir_and_fin() {
            return Err(TaskError::MultiFragmentResponse);
        }

        if response.header.iin.has_bad_request_error() {
            return Err(TaskError::RejectedByIin2(response.header.iin));
        }

        Ok(Some(response))
    }

    async fn run_read_task(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        mut task: ReadTask,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<Sequence, TaskError> {
        let result = self
            .execute_read_task(io, dest, &mut task, writer, reader)
            .await;

        let association = self.associations.get_mut(dest.link).ok();

        match result {
            Ok(_) => {
                if let Some(association) = association {
                    task.complete(association);
                } else {
                    task.on_task_error(None, TaskError::NoSuchAssociation(dest.link));
                }
            }
            Err(err) => task.on_task_error(association, err),
        }

        result
    }

    async fn execute_read_task(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        task: &mut ReadTask,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<Sequence, TaskError> {
        let mut seq = self.send_request(io, dest, task, writer).await?;
        let mut is_first = true;

        // read responses until we get a FIN or an error occurs
        loop {
            let timeout = self.associations.get_timeout(dest.link)?;
            let deadline = timeout.deadline_from_now();

            loop {
                tokio::select! {
                    _ = tokio::time::sleep_until(deadline) => {
                            tracing::warn!("no response within timeout: {}", timeout);
                            return Err(TaskError::ResponseTimeout);
                    }
                    x = reader.read(io, self.decode_level) => {
                        x?;
                        match reader.pop_response() {
                            Some(TransportResponse::Response(addr, response)) => {
                                self.notify_link_activity(addr.link);
                                let action = self.process_read_response(dest, is_first, seq, task, io, writer, addr, response).await?;
                                match action {
                                    // continue reading responses on the inner loop
                                    ReadResponseAction::Ignore => continue,
                                    // read task complete
                                    ReadResponseAction::Complete => return Ok(seq),
                                    // break to the outer loop and read another response
                                    ReadResponseAction::ReadNext => {
                                        is_first = false;
                                        seq = self.associations.get_mut(addr.link)?.increment_seq();
                                        break;
                                    }
                                }
                            }
                            Some(TransportResponse::LinkLayerMessage(msg)) => self.notify_link_activity(msg.source),
                            Some(TransportResponse::Error(err)) => return Err(err.into()),
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
        destination: FragmentAddr,
        is_first: bool,
        seq: Sequence,
        task: &mut ReadTask,
        io: &mut PhysLayer,
        writer: &mut TransportWriter,
        source: FragmentAddr,
        response: Response<'_>,
    ) -> Result<ReadResponseAction, TaskError> {
        if response.header.function.is_unsolicited() {
            self.handle_unsolicited(source, &response, io, writer)
                .await?;
            return Ok(ReadResponseAction::Ignore);
        }

        if source.link != destination.link {
            tracing::warn!(
                "Received response from {} while expecting response from {}",
                source.link,
                destination.link
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

        if response.header.iin.has_bad_request_error() {
            return Err(TaskError::RejectedByIin2(response.header.iin));
        }

        let association = self.associations.get_mut(destination.link)?;
        association.process_iin(response.header.iin);
        task.process_response(association, response.header, response.objects?)
            .await;

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
        source: FragmentAddr,
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
        source: FragmentAddr,
        response: &Response<'_>,
        io: &mut PhysLayer,
        writer: &mut TransportWriter,
    ) -> Result<(), LinkError> {
        let association = match self.associations.get_mut(source.link).ok() {
            Some(x) => x,
            None => {
                tracing::warn!(
                    "received unsolicited response from unknown address: {}",
                    source.link
                );
                return Ok(());
            }
        };

        association.process_iin(response.header.iin);

        let valid = association.handle_unsolicited_response(response).await;

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
        dest: FragmentAddr,
        seq: Sequence,
        writer: &mut TransportWriter,
    ) -> Result<(), LinkError> {
        let mut cursor = self.tx_buffer.write_cursor();
        write::confirm_solicited(seq, &mut cursor)?;
        writer
            .write(io, self.decode_level, dest, cursor.written())
            .await?;
        Ok(())
    }

    async fn confirm_unsolicited(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        seq: Sequence,
        writer: &mut TransportWriter,
    ) -> Result<(), LinkError> {
        let mut cursor = self.tx_buffer.write_cursor();
        write::confirm_unsolicited(seq, &mut cursor)?;

        writer
            .write(io, self.decode_level, dest, cursor.written())
            .await?;
        Ok(())
    }

    async fn send_request<U>(
        &mut self,
        io: &mut PhysLayer,
        addr: FragmentAddr,
        request: &U,
        writer: &mut TransportWriter,
    ) -> Result<Sequence, TaskError>
    where
        U: RequestWriter,
    {
        // format the request
        let association = self.associations.get_mut(addr.link)?;
        let seq = association.increment_seq();
        let mut cursor = self.tx_buffer.write_cursor();
        let mut hw =
            write::start_request(ControlField::request(seq), request.function(), &mut cursor)?;
        request.write(&mut hw)?;
        writer
            .write(io, self.decode_level, addr, cursor.written())
            .await?;
        Ok(seq)
    }
}

// Link status stuff
impl MasterSession {
    async fn run_link_status_task(
        &mut self,
        io: &mut PhysLayer,
        destination: FragmentAddr,
        writer: &mut TransportWriter,
        reader: &mut TransportReader,
    ) -> Result<(), TaskError> {
        // Send link status request
        tracing::info!("sending link status request (for {})", destination.link);
        writer
            .send_link_status_request(io, self.decode_level, destination)
            .await?;

        loop {
            let timeout = self.associations.get_timeout(destination.link)?;
            // Wait for something on the link
            tokio::select! {
                _ = tokio::time::sleep_until(timeout.deadline_from_now()) => {
                    tracing::warn!("no response within timeout: {}", timeout);
                    return Err(TaskError::ResponseTimeout);
                }
                x = reader.read(io, self.decode_level) => {
                    x?;
                    match reader.pop_response() {
                        Some(TransportResponse::Response(addr, response)) => {
                            self.notify_link_activity(addr.link);
                            self.handle_fragment_while_idle(io, writer, addr, response).await?;
                            return Err(TaskError::UnexpectedResponseHeaders);
                        }
                        Some(TransportResponse::LinkLayerMessage(msg)) => {
                            self.notify_link_activity(msg.source);
                            return Ok(());
                        }
                        Some(TransportResponse::Error(_)) => return Err(TaskError::UnexpectedResponseHeaders),
                        None => continue,
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

enum NextStep {
    Continue(NonReadTask),
    Complete(Sequence),
}
