// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::*;

impl MasterSession {
    pub(super) fn notify_link_activity(&mut self, source: EndpointAddress) {
        if let Ok(association) = self.associations.get_mut(source) {
            association.on_link_activity();
        }
    }
}

impl<T: Transport> MasterTask<T> {
    pub(super) async fn run_link_status_task(
        &mut self,
        io: &mut PhysLayer,
        destination: FragmentAddr,
    ) -> Result<(), TaskErrorKind> {
        // Send link status request
        tracing::info!("sending link status request (for {})", destination.link);
        let write =
            self.writer
                .send_link_status_request(io, self.session.decode_level, destination);
        unless_shutdown(&mut self.session.messages, write).await?;

        let deadline = Deadline::from_now(self.session.associations.get_timeout(destination.link)?);

        loop {
            match self
                .session
                .next_event(io, &mut self.reader, Some(deadline.at))
                .await?
            {
                Event::Timeout => return Err(deadline.expired()),
                Event::LinkLayer(LinkLayerMessage {
                    source,
                    message: LinkLayerMessageType::LinkStatusResponse,
                }) if source == destination.link => return Ok(()),
                Event::Fragment(source, response) => {
                    self.session
                        .handle_fragment_while_idle(io, &mut self.writer, source, response)
                        .await?;
                    if source.link == destination.link {
                        return Err(TaskErrorKind::UnexpectedResponseHeaders);
                    }
                }
                Event::Malformed(source, _) => {
                    if fails_task(source, destination) {
                        return Err(TaskErrorKind::UnexpectedResponseHeaders);
                    }
                }
                Event::LinkLayer(_) | Event::Message | Event::Nothing => {}
            }
        }
    }
}
