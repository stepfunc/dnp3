// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Master I/O orchestration and protocol session handling.
//!
//! [`MasterTask`] owns the transport resources and coordinates task execution, reads, and
//! deadlines. [`MasterSession`] owns association and application state, handles API messages,
//! and processes and formats application fragments. Session methods receive a reader or writer
//! when they need one. Keeping the session separate from transport storage lets it process a
//! response and send a confirmation while that response still borrows the reader's buffer.

mod link;
mod messages;
mod non_read;
pub(crate) mod read;
mod request;
mod response;
pub(crate) mod task;

use self::task::MasterTask;
use tracing::Instrument;

use crate::app::format::write;
use crate::app::parse::parser::Response;
use crate::app::{BufferSize, ControlField, FunctionCode, Iin, Iin1, Iin2, Sequence, Timeout};
use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::EndpointAddress;
use crate::master::association::{AssociationMap, Next};
use crate::master::error::TaskErrorKind;
use crate::master::extract::{Delivered, ProcessableResponse};
use crate::master::messages::{MasterMsg, Message};
use crate::master::tasks::command_no_ack::CommandNoAckTask;
use crate::master::tasks::{
    AssociationTask, NonReadTask, ReadTask, RequestWriter, SolicitedTask, Task,
};
use crate::master::{Association, MasterChannelConfig, TaskType};
use crate::phys::PhysLayer;
use crate::transport::{
    FragmentAddr, LinkLayerMessage, LinkLayerMessageType, RealTransport, Transport,
    TransportReader, TransportResponse, TransportResponseError, TransportWriter,
};
use crate::util::buffer::Buffer;
use crate::util::channel::Receiver;

use crate::app::parse::options::ParseOptions;
use crate::link::reader::LinkModes;
use crate::util::clock::{MonotonicClock, MonotonicTime, TokioClock};
use crate::util::lifecycle::{Enabled, RunError, StopReason};

/// Protocol state and message handling, borrowed independently of the task's transport storage.
struct MasterSession {
    enabled: Enabled,
    decode_level: DecodeLevel,
    associations: AssociationMap<TokioClock>,
    messages: Receiver<Message>,
    tx_buffer: Buffer,
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
    at: MonotonicTime,
    timeout: Timeout,
}

impl Deadline {
    fn from_now(timeout: Timeout, clock: TokioClock) -> Self {
        Self {
            at: timeout.deadline(clock.now()),
            timeout,
        }
    }

    fn expired(self) -> TaskErrorKind {
        tracing::warn!("no response within timeout: {}", self.timeout);
        TaskErrorKind::ResponseTimeout
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

impl MasterSession {
    fn new(
        initial_state: Enabled,
        decode_level: DecodeLevel,
        tx_buffer_size: BufferSize<249, 2048>,
        messages: Receiver<Message>,
        associations: AssociationMap<TokioClock>,
    ) -> Self {
        Self {
            enabled: initial_state,
            decode_level,
            associations,
            messages,
            tx_buffer: tx_buffer_size.create_buffer(),
        }
    }

    /// Wait for the next event: an API message, something read from the transport, or the deadline
    ///
    /// Returns an error only if shutdown or link layer error occurred.
    async fn next_event<'a, T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &'a mut TransportReader<T>,
        deadline: Option<MonotonicTime>,
    ) -> Result<Event<'a>, RunError> {
        let decode_level = self.decode_level;
        let clock = self.associations.clock();
        tokio::select! {
            result = self.process_message(true) => {
                result?;
                return Ok(Event::Message);
            }
            result = reader.read(io, decode_level) => result?,
            _ = clock.sleep_until(deadline) => return Ok(Event::Timeout),
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

    fn reset(&mut self, err: RunError) {
        self.associations.reset(err);
    }

    fn get_next_task(&mut self) -> Next<AssociationTask> {
        self.associations.next_task()
    }

    /// Tell the association's information callback that a task started
    fn notify_task_start(
        &mut self,
        dest: FragmentAddr,
        task_type: TaskType,
        function: FunctionCode,
    ) {
        if let Ok(assoc) = self.associations.get_mut(dest.link) {
            assoc.notify_task_start(task_type, function, assoc.seq());
        }
    }

    /// Tell the association's information callback how a task ended: with the function it started
    /// with and the sequence the completed task returned
    fn notify_task_end(
        &mut self,
        dest: FragmentAddr,
        task_type: TaskType,
        function: FunctionCode,
        result: Result<Sequence, TaskErrorKind>,
    ) -> Result<(), TaskErrorKind> {
        if let Ok(assoc) = self.associations.get_mut(dest.link) {
            match result {
                Ok(seq) => assoc.notify_task_success(task_type, function, seq),
                Err(err) => assoc.notify_task_fail(task_type, err),
            }
        }
        result.map(|_| ())
    }
}

impl<T: Transport> MasterTask<T> {
    /// Run the master until an error or shutdown occurs.
    async fn run_session(&mut self, io: &mut PhysLayer) -> RunError {
        loop {
            let result = match self.session.get_next_task() {
                Next::Now(task) => {
                    let id = task.details.get_id();
                    let address = task.dest.link.raw_value();
                    let span = tracing::info_span!("task", "type" = ?id, "dest" = address);
                    std::pin::pin!(self.run_task(io, task).instrument(span)).await
                }
                Next::NotBefore(time) => self.idle(Some(time), io).await,
                Next::None => self.idle(None, io).await,
            };

            if let Err(err) = result {
                self.session.reset(err);

                if RunError::Stop(StopReason::Shutdown) == err {
                    self.session.messages.close_and_drain();
                }

                return err;
            }
        }
    }

    /// Wait until a message is received, a response is received, or we reach the time, if any.
    ///
    /// Returns an error only if shutdown or link layer error occurred.
    async fn idle(
        &mut self,
        until: Option<MonotonicTime>,
        io: &mut PhysLayer,
    ) -> Result<(), RunError> {
        loop {
            match self.session.next_event(io, &mut self.reader, until).await? {
                Event::Fragment(source, response) => {
                    return self
                        .session
                        .handle_fragment_while_idle(io, &mut self.writer, source, response)
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

    /// Run a specific task.
    ///
    /// Returns an error only if shutdown or link layer error occurred.
    async fn run_task(
        &mut self,
        io: &mut PhysLayer,
        task: AssociationTask,
    ) -> Result<(), RunError> {
        let result = match task.details {
            Task::Solicited(t) => {
                let (task_type, function) = t.identity();
                self.session
                    .notify_task_start(task.dest, task_type, function);
                let res = match t {
                    SolicitedTask::Read(t) => self.run_read_task(io, task.dest, t).await,
                    SolicitedTask::NonRead(t) => {
                        std::pin::pin!(self.run_non_read_task(io, task.dest, t)).await
                    }
                };
                self.session
                    .notify_task_end(task.dest, task_type, function, res)
            }
            Task::NoResponse(t) => {
                let function = t.function();
                self.session
                    .notify_task_start(task.dest, TaskType::Command, function);
                let res = self
                    .session
                    .run_no_response_task(io, task.dest, t, &mut self.writer)
                    .await;
                self.session
                    .notify_task_end(task.dest, TaskType::Command, function, res)
            }
            Task::LinkStatus(promise) => {
                let res = self.run_link_status_task(io, task.dest).await;
                promise.complete(res.map_err(Into::into));
                res
            }
        };

        // if a task error occurs, if might be a run error
        match result {
            Ok(()) => Ok(()),
            Err(err) => match err {
                TaskErrorKind::Shutdown => Err(RunError::Stop(StopReason::Shutdown)),
                TaskErrorKind::Disabled => Err(RunError::Stop(StopReason::Disable)),
                TaskErrorKind::Link(err) => Err(RunError::Link(err)),
                TaskErrorKind::TooManyRequests
                | TaskErrorKind::Transport
                | TaskErrorKind::RejectedByIin2(_)
                | TaskErrorKind::MalformedResponse(_)
                | TaskErrorKind::UnexpectedResponseHeaders
                | TaskErrorKind::NonFinWithoutCon
                | TaskErrorKind::NeverReceivedFir
                | TaskErrorKind::UnexpectedFir
                | TaskErrorKind::MultiFragmentResponse
                | TaskErrorKind::ResponseTimeout
                | TaskErrorKind::WriteError
                | TaskErrorKind::BadEncoding(_)
                | TaskErrorKind::NoSuchAssociation(_)
                | TaskErrorKind::NoConnection
                | TaskErrorKind::SystemTimeNotAvailable => Ok(()),
            },
        }
    }
}
