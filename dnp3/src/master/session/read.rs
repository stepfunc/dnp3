// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::*;

/// What the fragments of a read reported and delivered, which decides its outcome when it completes
#[derive(Default)]
pub(crate) struct ReadOutcome {
    /// the IIN2.0, IIN2.1 and IIN2.2 bits of every fragment
    errors: Iin2,
    /// the IIN1 bits of the last fragment
    iin1: Iin1,
    /// what the fragments delivered to the handler
    delivered: Delivered,
}

impl ReadOutcome {
    pub(crate) fn add(&mut self, iin: Iin, delivered: Delivered) {
        self.errors |= iin.bad_request_errors();
        self.iin1 = iin.iin1;
        self.delivered.merge(delivered);
    }

    /// True if the read returned events, and no fragment reported an IIN2 request error
    /// (IIN2.0 to IIN2.2), so the outstation didn't reject any part of the request
    pub(crate) fn events_without_request_errors(&self) -> bool {
        self.delivered.events && self.errors == Iin2::default()
    }

    /// A read with errors still succeeds if it delivered data, or if it's an automatic task
    pub(crate) fn result(&self, task: &ReadTask) -> Result<(), TaskError> {
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

impl MasterSession {
    pub(super) async fn run_read_task<T: Transport>(
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
}
