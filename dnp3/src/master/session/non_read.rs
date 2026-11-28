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
    ) -> Result<Sequence, TaskErrorKind> {
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

    async fn poll_non_read_response<'a, 'b, T: Transport>(
        &'b mut self,
        io: &mut PhysLayer,
        reader: &'a mut TransportReader<T>,
        writer: &mut TransportWriter<T>,
        dest: FragmentAddr,
        seq: Sequence,
        deadline: Deadline,
    ) -> Result<Option<(&'b mut Association<TokioClock>, Response<'a>)>, TaskErrorKind> {
        let (source, response) = match self.next_event(io, reader, Some(deadline.at)).await? {
            Event::Timeout => return Err(deadline.expired()),
            Event::Malformed(source, err) if fails_task(source, dest) => return Err(err.into()),
            Event::Fragment(source, response) => (source, response),
            Event::Malformed(..) | Event::Message | Event::LinkLayer(_) | Event::Nothing => {
                return Ok(None);
            }
        };

        let Some(response) = self
            .filter_solicited(io, writer, dest, seq, source, response)
            .await?
        else {
            return Ok(None);
        };

        if !response.header.control.is_fir_and_fin() {
            return Err(TaskErrorKind::MultiFragmentResponse);
        }

        // a response for a removed association isn't processed, so it isn't confirmed
        self.associations.get_mut(dest.link)?;

        // confirm before handling the response, so the confirm precedes any next request
        if response.header.control.con && response.objects.is_ok() {
            self.send_confirm(io, dest, ControlField::request(seq), writer)
                .await?;
        }

        let association = self.associations.get_mut(dest.link)?;
        association.process_iin(response.header.iin);
        if response.header.iin.has_bad_request_error() {
            return Err(TaskErrorKind::RejectedByIin2(response.header.iin));
        }

        Ok(Some((association, response)))
    }
}

impl<T: Transport> MasterTask<T> {
    pub(super) async fn run_non_read_task(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        task: NonReadTask,
    ) -> Result<Sequence, TaskErrorKind> {
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
    ) -> Result<NextStep, TaskErrorKind> {
        let (seq, deadline) = match self.send_non_read_request(io, dest, &task).await {
            Ok(started) => started,
            Err(err) => return Err(self.fail_non_read(dest, task, err)),
        };

        loop {
            match self
                .session
                .poll_non_read_response(io, &mut self.reader, &mut self.writer, dest, seq, deadline)
                .await
            {
                Ok(Some((association, response))) => {
                    return match association.handle_non_read_response(task, response).await? {
                        Some(next) => Ok(NextStep::Continue(next)),
                        None => Ok(NextStep::Complete(seq)),
                    };
                }
                Ok(None) => {}
                Err(err) => return Err(self.fail_non_read(dest, task, err)),
            }
        }
    }

    async fn send_non_read_request(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        task: &NonReadTask,
    ) -> Result<(Sequence, Deadline), TaskErrorKind> {
        let seq = self
            .session
            .send_request(io, dest, task, &mut self.writer)
            .await?;
        let deadline = Deadline::from_now(
            self.session.associations.get_timeout(dest.link)?,
            self.session.associations.clock(),
        );
        Ok((seq, deadline))
    }

    fn fail_non_read(
        &mut self,
        dest: FragmentAddr,
        task: NonReadTask,
        err: TaskErrorKind,
    ) -> TaskErrorKind {
        match self.session.associations.get_mut(dest.link) {
            Ok(association) => association.fail_non_read(task, err),
            Err(_) => task.on_task_error(err),
        }
        err
    }
}
