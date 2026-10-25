// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::*;

enum NextStep {
    Continue(NonReadTask),
    Complete(Sequence),
}

impl MasterSession {
    pub(super) async fn run_no_response_task<T: Transport>(
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
}

impl<T: Transport> MasterTask<T> {
    pub(super) async fn run_non_read_task(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        task: NonReadTask,
    ) -> Result<Sequence, TaskError> {
        let mut next = NextStep::Continue(task);

        loop {
            next = match next {
                NextStep::Continue(task) => {
                    std::pin::pin!(self.run_single_non_read_task(io, dest, task)).await?
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
    ) -> Result<NextStep, TaskError> {
        let err = 'failed: {
            let seq = match self
                .session
                .send_request(io, dest, &task, &mut self.writer)
                .await
            {
                Ok(seq) => seq,
                Err(err) => break 'failed err,
            };

            let deadline = match self.session.associations.get_timeout(dest.link) {
                Ok(timeout) => Deadline::from_now(timeout),
                Err(err) => break 'failed err,
            };

            loop {
                let (source, response) = match self
                    .session
                    .next_event(io, &mut self.reader, Some(deadline.at))
                    .await
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
                    .session
                    .filter_solicited(io, &mut self.writer, dest, seq, source, response)
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
                if let Err(err) = self.session.associations.get_mut(dest.link) {
                    break 'failed err.into();
                }

                // confirm before handling the response, so the confirm precedes any next request
                if response.header.control.con && response.objects.is_ok() {
                    if let Err(err) = self
                        .session
                        .send_confirm(io, dest, ControlField::request(seq), &mut self.writer)
                        .await
                    {
                        break 'failed err.into();
                    }
                }

                let association = match self.session.associations.get_mut(dest.link) {
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

        task.on_task_error(self.session.associations.get_mut(dest.link).ok(), err);
        Err(err)
    }
}
