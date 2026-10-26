// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::*;

/// Owns transport resources and orchestrates master I/O, task execution, and deadlines.
///
/// Protocol state and processing live on the separate session field so they can run while a
/// fragment borrows the reader.
pub(crate) struct MasterTask<T: Transport = RealTransport> {
    pub(super) session: MasterSession,
    pub(super) reader: TransportReader<T>,
    pub(super) writer: TransportWriter<T>,
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
        let ret = self.run_session(io).await;

        self.writer.reset();
        self.reader.reset();

        ret
    }

    pub(crate) async fn process_next_message(&mut self) -> Result<(), StopReason> {
        self.session.process_next_message().await
    }
}
