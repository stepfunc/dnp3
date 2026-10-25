// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::*;

impl MasterSession {
    /// Pass on a solicited response from `destination` with sequence `seq`. Unsolicited responses
    /// are processed and confirmed here, and responses from other sources or with other sequence
    /// numbers are ignored.
    pub(super) async fn filter_solicited<'a, T: Transport>(
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

    pub(super) async fn handle_fragment_while_idle<T: Transport>(
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
