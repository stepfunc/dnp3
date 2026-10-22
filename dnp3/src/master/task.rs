use tracing::Instrument;

use crate::app::format::write;
use crate::app::parse::parser::Response;
use crate::app::{BufferSize, ControlField, FunctionCode, Iin, Iin1, Iin2, Sequence, Timeout};
use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::EndpointAddress;
use crate::master::association::{AssociationMap, Next};
use crate::master::error::TaskError;
use crate::master::extract::{Delivered, ProcessableResponse};
use crate::master::messages::{MasterMsg, Message};
use crate::master::tasks::command_no_ack::CommandNoAckTask;
use crate::master::tasks::{AppTask, AssociationTask, NonReadTask, ReadTask, RequestWriter, Task};
use crate::master::{Association, MasterChannelConfig};
use crate::transport::{
    FragmentAddr, LinkLayerMessage, LinkLayerMessageType, RealTransport, Transport,
    TransportReader, TransportResponse, TransportResponseError, TransportWriter,
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
        config: MasterChannelConfig,
        messages: Receiver<Message>,
    ) -> Self {
        Self::with_associations(
            initial_state,
            AssociationMap::new(),
            link_modes,
            config,
            messages,
        )
    }

    pub(crate) fn with_associations(
        enabled: Enabled,
        associations: AssociationMap,
        link_modes: LinkModes,
        config: MasterChannelConfig,
        messages: Receiver<Message>,
    ) -> Self {
        Self::with_transport(enabled, associations, link_modes, config, messages)
    }

    pub(crate) fn seed_link(&mut self, seed_data: &[u8]) -> Result<(), LinkError> {
        self.reader.seed_link(seed_data)
    }

    /// Parse with `options` instead of those read when the task was created
    pub(crate) fn set_parse_options(&mut self, options: ParseOptions) {
        self.reader.set_parse_options(options)
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
    /// what the fragments delivered to the handler
    delivered: Delivered,
}

impl ReadOutcome {
    fn add(&mut self, iin: Iin, delivered: Delivered) {
        self.errors |= iin.bad_request_errors();
        self.iin1 = iin.iin1;
        self.delivered.merge(delivered);
    }

    /// True if the read returned events, and no fragment reported an IIN2 request error
    /// (IIN2.0 to IIN2.2), so the outstation didn't reject any part of the request
    fn events_without_request_errors(&self) -> bool {
        self.delivered.events && self.errors == Iin2::default()
    }

    /// A read with errors still succeeds if it delivered data, or if it's an automatic task
    fn result(&self, task: &ReadTask) -> Result<(), TaskError> {
        if self.errors == Iin2::default() {
            return Ok(());
        }
        if self.delivered.values || !task.fails_without_data() {
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
    ReadNext,
    Complete,
}

/// What happened while the session waited
enum Event<'a> {
    /// the deadline passed
    Timeout,
    /// an API message was processed
    Message,
    /// a fragment arrived, and link activity was credited to its source
    Fragment(FragmentAddr, Response<'a>),
    /// a link-layer message arrived, and link activity was credited to its source
    LinkLayer(LinkLayerMessage),
    /// a fragment arrived from this source that isn't a valid response, and no link activity was
    /// credited
    Malformed(FragmentAddr, TransportResponseError),
    /// the read completed without a fragment or link-layer message
    Nothing,
}

/// When a task stops waiting for a response
#[derive(Copy, Clone)]
struct Deadline {
    at: Instant,
    timeout: Timeout,
}

impl Deadline {
    fn from_now(timeout: Timeout) -> Self {
        Self {
            at: timeout.deadline_from_now(),
            timeout,
        }
    }

    fn expired(self) -> TaskError {
        tracing::warn!("no response within timeout: {}", self.timeout);
        TaskError::ResponseTimeout
    }
}

/// Whether a malformed fragment from `source` fails a task waiting on `destination`. One from
/// another source is dropped.
fn fails_task(source: FragmentAddr, destination: FragmentAddr) -> bool {
    if source.link == destination.link {
        return true;
    }
    tracing::debug!(
        "ignoring malformed fragment from {} while expecting response from {}",
        source.link,
        destination.link
    );
    false
}

async fn sleep_until(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
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
                Next::NotBefore(time) => self.idle(Some(time), io, writer, reader).await,
                Next::None => self.idle(None, io, writer, reader).await,
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

    /// Wait until a message is received, a response is received, or we reach the time, if any.
    ///
    /// Returns an error only if shutdown or link layer error occurred.
    async fn idle<T: Transport>(
        &mut self,
        until: Option<Instant>,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
        reader: &mut TransportReader<T>,
    ) -> Result<(), RunError> {
        loop {
            match self.next_event(io, reader, until).await? {
                Event::Fragment(source, response) => {
                    return self
                        .handle_fragment_while_idle(io, writer, source, response)
                        .await
                }
                Event::LinkLayer(_) => {}
                // we need to recheck the tasks
                Event::Timeout | Event::Message | Event::Malformed(..) | Event::Nothing => {
                    return Ok(())
                }
            }
        }
    }

    /// Wait for the next event: an API message, something read from the transport, or the deadline
    ///
    /// Returns an error only if shutdown or link layer error occurred.
    async fn next_event<'a, T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &'a mut TransportReader<T>,
        deadline: Option<Instant>,
    ) -> Result<Event<'a>, RunError> {
        let decode_level = self.decode_level;
        tokio::select! {
            result = self.process_message(true) => {
                result?;
                return Ok(Event::Message);
            }
            result = reader.read(io, decode_level) => result?,
            _ = sleep_until(deadline) => return Ok(Event::Timeout),
        }

        let event = match reader.pop_response(decode_level.application) {
            Some(TransportResponse::Response(source, response)) => {
                self.notify_link_activity(source.link);
                Event::Fragment(source, response)
            }
            Some(TransportResponse::LinkLayerMessage(msg)) => {
                self.notify_link_activity(msg.source);
                Event::LinkLayer(msg)
            }
            Some(TransportResponse::Error(source, err)) => Event::Malformed(source, err),
            None => Event::Nothing,
        };
        Ok(event)
    }

    /// Pass on a solicited response from `destination` with sequence `seq`. Unsolicited responses
    /// are processed and confirmed here, and responses from other sources or with other sequence
    /// numbers are ignored.
    async fn filter_solicited<'a, T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
        destination: FragmentAddr,
        seq: Sequence,
        source: FragmentAddr,
        response: Response<'a>,
    ) -> Result<Option<Response<'a>>, LinkError> {
        if response.header.function.is_unsolicited() {
            self.handle_unsolicited(source, &response, io, writer)
                .await?;
            return Ok(None);
        }

        if source.link != destination.link {
            tracing::debug!(
                "Received response from {} while expecting response from {}",
                source.link,
                destination.link
            );
            return Ok(None);
        }

        if response.header.control.seq != seq {
            tracing::warn!(
                "response with seq: {} doesn't match expected seq: {}",
                response.header.control.seq.value(),
                seq.value()
            );
            return Ok(None);
        }

        Ok(Some(response))
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
        let err = 'failed: {
            let seq = match self.send_request(io, dest, &task, writer).await {
                Ok(seq) => seq,
                Err(err) => break 'failed err,
            };

            let deadline = match self.associations.get_timeout(dest.link) {
                Ok(timeout) => Deadline::from_now(timeout),
                Err(err) => break 'failed err,
            };

            loop {
                let (source, response) = match self.next_event(io, reader, Some(deadline.at)).await
                {
                    Err(err) => break 'failed err.into(),
                    Ok(Event::Timeout) => break 'failed deadline.expired(),
                    Ok(Event::Malformed(source, err)) => {
                        if fails_task(source, dest) {
                            break 'failed err.into();
                        }
                        continue;
                    }
                    Ok(Event::Fragment(source, response)) => (source, response),
                    Ok(Event::Message | Event::LinkLayer(_) | Event::Nothing) => continue,
                };

                let response = match self
                    .filter_solicited(io, writer, dest, seq, source, response)
                    .await
                {
                    Err(err) => break 'failed err.into(),
                    Ok(None) => continue,
                    Ok(Some(response)) => response,
                };

                if !response.header.control.is_fir_and_fin() {
                    break 'failed TaskError::MultiFragmentResponse;
                }

                // a response for a removed association isn't processed, so it isn't confirmed
                if let Err(err) = self.associations.get_mut(dest.link) {
                    break 'failed err.into();
                }

                // confirm before handling the response, so the confirm precedes any next request
                if response.header.control.con && response.objects.is_ok() {
                    if let Err(err) = self
                        .send_confirm(io, dest, ControlField::request(seq), writer)
                        .await
                    {
                        break 'failed err.into();
                    }
                }

                let association = match self.associations.get_mut(dest.link) {
                    Ok(association) => association,
                    Err(err) => break 'failed err.into(),
                };

                association.process_iin(response.header.iin);
                if response.header.iin.has_bad_request_error() {
                    break 'failed TaskError::RejectedByIin2(response.header.iin);
                }

                return match task.handle_response(association, response).await? {
                    Some(next) => Ok(NextStep::Continue(next)),
                    None => Ok(NextStep::Complete(seq)),
                };
            }
        };

        task.on_task_error(self.associations.get_mut(dest.link).ok(), err);
        Err(err)
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
            Ok((seq, events_without_request_errors)) => {
                if let Some(association) = association {
                    task.complete(association, events_without_request_errors);
                } else {
                    task.on_task_error(None, TaskError::NoSuchAssociation(dest.link));
                }
                Ok(seq)
            }
            Err(err) => {
                task.on_task_error(association, err);
                Err(err)
            }
        }
    }

    /// Run a read, returning the sequence of the last response and whether the read returned
    /// events without an IIN2 request error
    async fn execute_read_task<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        task: &mut ReadTask,
        writer: &mut TransportWriter<T>,
        reader: &mut TransportReader<T>,
    ) -> Result<(Sequence, bool), TaskError> {
        let mut seq = self.send_request(io, dest, task, writer).await?;
        let mut is_first = true;
        let mut outcome = ReadOutcome::default();

        // read responses until we get a FIN or an error occurs
        loop {
            let deadline = Deadline::from_now(self.associations.get_timeout(dest.link)?);

            let action = loop {
                let (source, response) =
                    match self.next_event(io, reader, Some(deadline.at)).await? {
                        Event::Timeout => return Err(deadline.expired()),
                        Event::Malformed(source, err) => {
                            if fails_task(source, dest) {
                                return Err(err.into());
                            }
                            continue;
                        }
                        Event::Fragment(source, response) => (source, response),
                        Event::Message | Event::LinkLayer(_) | Event::Nothing => continue,
                    };

                if let Some(response) = self
                    .filter_solicited(io, writer, dest, seq, source, response)
                    .await?
                {
                    break self
                        .process_read_response(
                            dest,
                            is_first,
                            seq,
                            task,
                            &mut outcome,
                            io,
                            writer,
                            response,
                        )
                        .await?;
                }
            };

            match action {
                // read task complete
                ReadResponseAction::Complete => {
                    outcome.result(task)?;
                    return Ok((seq, outcome.events_without_request_errors()));
                }
                // read another response
                ReadResponseAction::ReadNext => {
                    is_first = false;
                    seq = self.associations.get_mut(dest.link)?.increment_seq();
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
        response: Response<'_>,
    ) -> Result<ReadResponseAction, TaskError> {
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
        let objects = ProcessableResponse::new(response.objects).inspect_err(|err| {
            tracing::warn!("discarding response: {err}");
        })?;
        let delivered = task
            .process_response(association, response.header, objects)
            .await;
        outcome.add(response.header.iin, delivered);

        if response.header.control.con {
            self.send_confirm(io, destination, ControlField::request(seq), writer)
                .await?;
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
            let control = ControlField::unsolicited(response.header.control.seq);
            self.send_confirm(io, source, control, writer).await?;
        }

        Ok(())
    }
}

// Sending methods
impl MasterSession {
    /// Send a CONFIRM with this control field: solicited or unsolicited, and its sequence
    async fn send_confirm<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        control: ControlField,
        writer: &mut TransportWriter<T>,
    ) -> Result<(), LinkError> {
        let mut cursor = self.tx_buffer.write_cursor();
        write::start_request(control, FunctionCode::Confirm, &mut cursor)?;
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

        let deadline = Deadline::from_now(self.associations.get_timeout(destination.link)?);

        loop {
            match self.next_event(io, reader, Some(deadline.at)).await? {
                Event::Timeout => return Err(deadline.expired()),
                Event::LinkLayer(LinkLayerMessage {
                    source,
                    message: LinkLayerMessageType::LinkStatusResponse,
                }) if source == destination.link => return Ok(()),
                Event::Fragment(source, response) => {
                    self.handle_fragment_while_idle(io, writer, source, response)
                        .await?;
                    if source.link == destination.link {
                        return Err(TaskError::UnexpectedResponseHeaders);
                    }
                }
                Event::Malformed(source, _) => {
                    if fails_task(source, destination) {
                        return Err(TaskError::UnexpectedResponseHeaders);
                    }
                }
                Event::LinkLayer(_) | Event::Message | Event::Nothing => {}
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
