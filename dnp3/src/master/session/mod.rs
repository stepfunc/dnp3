// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Master session orchestration and its private task, request, and response handling.

mod link;
mod messages;
mod non_read;
pub(crate) mod read;
mod request;
mod response;
pub(crate) mod task;

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
use crate::phys::PhysLayer;
use crate::transport::{
    FragmentAddr, LinkLayerMessage, LinkLayerMessageType, RealTransport, Transport,
    TransportReader, TransportResponse, TransportResponseError, TransportWriter,
};
use crate::util::buffer::Buffer;
use crate::util::channel::Receiver;

use crate::app::parse::options::ParseOptions;
use crate::link::reader::LinkModes;
use crate::util::lifecycle::{Enabled, RunError, StopReason};
use tokio::time::Instant;

struct MasterSession {
    enabled: Enabled,
    decode_level: DecodeLevel,
    associations: AssociationMap,
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
                    let span = tracing::info_span!("task", "type" = ?id, "dest" = address);
                    std::pin::pin!(self.run_task(io, task, writer, reader).instrument(span)).await
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
                        std::pin::pin!(self.run_non_read_task(io, task.dest, t, writer, reader))
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
                | TaskError::NoConnection
                | TaskError::SystemTimeNotAvailable => Ok(()),
            },
        }
    }

    fn get_next_task(&mut self) -> Next<AssociationTask> {
        self.associations.next_task()
    }
}
