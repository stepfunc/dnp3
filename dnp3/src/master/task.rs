use tracing::Instrument;

use crate::app::format::write;
use crate::app::parse::parser::Response;
use crate::app::{BufferSize, ControlField, Iin, Iin1, Iin2, Sequence};
use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::EndpointAddress;
use crate::master::association::{AssociationMap, Next};
use crate::master::error::TaskError;
use crate::master::extract::validate_response;
use crate::master::messages::{MasterMsg, Message};
use crate::master::tasks::command_no_ack::CommandNoAckTask;
use crate::master::tasks::{AppTask, AssociationTask, NonReadTask, ReadTask, RequestWriter, Task};
use crate::master::{Association, MasterChannelConfig};
use crate::transport::{
    FragmentAddr, RealTransport, Transport, TransportReader, TransportResponse, TransportWriter,
};
use crate::util::buffer::Buffer;
use crate::util::channel::Receiver;
use crate::util::phys::PhysLayer;

use crate::app::parse::options::ParseOptions;
use crate::link::reader::LinkModes;
use crate::util::session::{Enabled, RunError, StopReason};
use tokio::time::Instant;

/// combines the session with transport stuff
pub(crate) struct MasterTask<T: Transport = RealTransport> {
    session: MasterSession,
    reader: TransportReader<T>,
    writer: TransportWriter<T>,
}

impl MasterTask {
    pub(crate) fn new(
        initial_state: Enabled,
        link_modes: LinkModes,
        parse_options: ParseOptions,
        config: MasterChannelConfig,
        messages: Receiver<Message>,
    ) -> Self {
        Self::with_associations(
            initial_state,
            AssociationMap::new(),
            link_modes,
            parse_options,
            config,
            messages,
        )
    }

    pub(crate) fn with_associations(
        enabled: Enabled,
        associations: AssociationMap,
        link_modes: LinkModes,
        parse_options: ParseOptions,
        config: MasterChannelConfig,
        messages: Receiver<Message>,
    ) -> Self {
        Self::with_transport(
            enabled,
            associations,
            link_modes,
            parse_options,
            config,
            messages,
        )
    }

    pub(crate) fn seed_link(&mut self, seed_data: &[u8]) -> Result<(), LinkError> {
        self.reader.seed_link(seed_data)
    }
}

#[cfg(test)]
impl MasterTask<crate::transport::mock::MockTransport> {
    pub(crate) fn set_rx_frame_info(&mut self, info: crate::link::header::FrameInfo) {
        self.reader.set_rx_frame_info(info);
    }

    pub(crate) fn rx_frame_info(&self) -> crate::transport::mock::reader::RxFrameInfo {
        self.reader.rx_frame_info()
    }
}

impl<T: Transport> MasterTask<T> {
    pub(crate) fn with_transport(
        enabled: Enabled,
        associations: AssociationMap,
        link_modes: LinkModes,
        parse_options: ParseOptions,
        config: MasterChannelConfig,
        messages: Receiver<Message>,
    ) -> Self {
        let session = MasterSession::new(
            enabled,
            config.decode_level,
            config.tx_buffer_size,
            messages,
            associations,
        );
        let (reader, writer) = crate::transport::create_transport_layer(
            link_modes,
            parse_options,
            crate::link::LinkRole::Master,
            config.master_address,
            config.rx_buffer_size.value(),
        );
        Self {
            session,
            reader,
            writer,
        }
    }

    pub(crate) fn enabled(&self) -> Enabled {
        self.session.enabled
    }

    #[cfg(feature = "unstable")]
    pub(crate) fn start(&mut self, now: Instant) {
        self.session.associations.start(now);
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

/// What the fragments of a read reported and delivered, which decides its outcome when it completes
#[derive(Default)]
struct ReadOutcome {
    /// the IIN2.0, IIN2.1 and IIN2.2 bits of every fragment
    errors: Iin2,
    /// the IIN1 bits of the last fragment
    iin1: Iin1,
    /// true if any fragment delivered measurements to the handler
    delivered: bool,
}

impl ReadOutcome {
    fn add(&mut self, iin: Iin, delivered: bool) {
        self.errors |= iin.bad_request_errors();
        self.iin1 = iin.iin1;
        self.delivered |= delivered;
    }

    /// A read with errors still succeeds if it delivered data, or if it's an automatic task
    fn result(&self, task: &ReadTask) -> Result<(), TaskError> {
        if self.errors == Iin2::default() {
            return Ok(());
        }
        if self.delivered || !task.fails_without_data() {
            tracing::warn!(
                "read completed, but the outstation reported: {}",
                self.errors
            );
            return Ok(());
        }
        Err(TaskError::RejectedByIin2(Iin::new(self.iin1, self.errors)))
    }
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
        associations: AssociationMap,
    ) -> Self {
        Self {
            enabled: initial_state,
            decode_level,
            associations,
            messages,
            tx_buffer: tx_buffer_size.create_buffer(),
        }
    }

    /// Run the master until an error or shutdown occurs.
    async fn run<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
        reader: &mut TransportReader<T>,
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
    async fn idle_forever<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
        reader: &mut TransportReader<T>,
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
    async fn idle_until<T: Transport>(
        &mut self,
        instant: Instant,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
        reader: &mut TransportReader<T>,
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
    async fn run_task<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        task: AssociationTask,
        writer: &mut TransportWriter<T>,
        reader: &mut TransportReader<T>,
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
                    AppTask::NoResponse(t) => {
                        self.run_no_response_task(io, task.dest, t, writer).await
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
                TaskError::TooManyRequests
                | TaskError::Transport
                | TaskError::RejectedByIin2(_)
                | TaskError::MalformedResponse(_)
                | TaskError::UnexpectedResponseHeaders
                | TaskError::NonFinWithoutCon
                | TaskError::NeverReceivedFir
                | TaskError::UnexpectedFir
                | TaskError::MultiFragmentResponse
                | TaskError::ResponseTimeout
                | TaskError::WriteError
                | TaskError::BadEncoding(_)
                | TaskError::NoSuchAssociation(_)
                | TaskError::NoConnection => Ok(()),
            },
        }
    }

    async fn run_no_response_task<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        task: CommandNoAckTask,
        writer: &mut TransportWriter<T>,
    ) -> Result<Sequence, TaskError> {
        match self.send_request(io, dest, &task, writer).await {
            Ok(seq) => {
                task.complete();
                Ok(seq)
            }
            Err(err) => {
                task.on_task_error(err);
                Err(err)
            }
        }
    }

    async fn run_non_read_task<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        task: NonReadTask,
        writer: &mut TransportWriter<T>,
        reader: &mut TransportReader<T>,
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

    async fn run_single_non_read_task<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        task: NonReadTask,
        writer: &mut TransportWriter<T>,
        reader: &mut TransportReader<T>,
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
                            self.notify_link_activity(source.link);

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
                                            if response.header.iin.has_bad_request_error() {
                                                let err = TaskError::RejectedByIin2(response.header.iin);
                                                task.on_task_error(Some(association), err);
                                                return Err(err);
                                            }
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

    async fn validate_non_read_response<'a, T: Transport>(
        &mut self,
        destination: FragmentAddr,
        seq: Sequence,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
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

        Ok(Some(response))
    }

    async fn run_read_task<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        mut task: ReadTask,
        writer: &mut TransportWriter<T>,
        reader: &mut TransportReader<T>,
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

    async fn execute_read_task<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        task: &mut ReadTask,
        writer: &mut TransportWriter<T>,
        reader: &mut TransportReader<T>,
    ) -> Result<Sequence, TaskError> {
        let mut seq = self.send_request(io, dest, task, writer).await?;
        let mut is_first = true;
        let mut outcome = ReadOutcome::default();

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
                                let action = self.process_read_response(dest, is_first, seq, task, &mut outcome, io, writer, addr, response).await?;
                                match action {
                                    // continue reading responses on the inner loop
                                    ReadResponseAction::Ignore => continue,
                                    // read task complete
                                    ReadResponseAction::Complete => return outcome.result(task).map(|()| seq),
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

    #[expect(
        clippy::too_many_arguments,
        reason = "processing a read response needs the request, the response and the session state"
    )]
    async fn process_read_response<T: Transport>(
        &mut self,
        destination: FragmentAddr,
        is_first: bool,
        seq: Sequence,
        task: &mut ReadTask,
        outcome: &mut ReadOutcome,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
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

        // a response with IIN2 errors is still processed, and the errors decide the outcome at the end
        let association = self.associations.get_mut(destination.link)?;
        association.process_iin(response.header.iin);
        let objects = validate_response(response.objects).inspect_err(|err| {
            tracing::warn!("discarding response: {err}");
        })?;
        let delivered = task
            .process_response(association, response.header, objects)
            .await;
        outcome.add(response.header.iin, delivered);

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
    async fn handle_fragment_while_idle<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
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

    async fn handle_unsolicited<T: Transport>(
        &mut self,
        source: FragmentAddr,
        response: &Response<'_>,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
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
    async fn confirm_solicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        seq: Sequence,
        writer: &mut TransportWriter<T>,
    ) -> Result<(), LinkError> {
        let mut cursor = self.tx_buffer.write_cursor();
        write::confirm_solicited(seq, &mut cursor)?;
        writer
            .write(io, self.decode_level, dest, cursor.written())
            .await?;
        Ok(())
    }

    async fn confirm_unsolicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        seq: Sequence,
        writer: &mut TransportWriter<T>,
    ) -> Result<(), LinkError> {
        let mut cursor = self.tx_buffer.write_cursor();
        write::confirm_unsolicited(seq, &mut cursor)?;

        writer
            .write(io, self.decode_level, dest, cursor.written())
            .await?;
        Ok(())
    }

    async fn send_request<T: Transport, U>(
        &mut self,
        io: &mut PhysLayer,
        addr: FragmentAddr,
        request: &U,
        writer: &mut TransportWriter<T>,
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
    async fn run_link_status_task<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        destination: FragmentAddr,
        writer: &mut TransportWriter<T>,
        reader: &mut TransportReader<T>,
    ) -> Result<(), TaskError> {
        // Send link status request
        tracing::info!("sending link status request (for {})", destination.link);
        writer
            .send_link_status_request(io, self.decode_level, destination)
            .await?;

        let timeout = self.associations.get_timeout(destination.link)?;
        let deadline = timeout.deadline_from_now();

        loop {
            // Wait for something on the link
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
