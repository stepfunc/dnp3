use crate::app::Shutdown;
use crate::decode::DecodeLevel;
use crate::link::LinkErrorMode;
use crate::outstation::config::*;
use crate::outstation::database::{DatabaseHandle, EventBufferConfig};
use crate::outstation::session::{OutstationSession, RunError};
use crate::outstation::traits::{ControlHandler, OutstationApplication, OutstationInformation};
use crate::outstation::OutstationHandle;
use crate::transport::{TransportReader, TransportWriter};
use crate::util::phys::PhysLayer;

pub(crate) enum ConfigurationChange {
    SetDecodeLevel(DecodeLevel),
}

impl From<ConfigurationChange> for OutstationMessage {
    fn from(x: ConfigurationChange) -> Self {
        OutstationMessage::Configuration(x)
    }
}

pub(crate) enum OutstationMessage {
    Shutdown,
    Configuration(ConfigurationChange),
}

pub(crate) struct OutstationTask {
    session: OutstationSession,
    reader: TransportReader,
    writer: TransportWriter,
    database: DatabaseHandle,
}

impl OutstationTask {
    /// create an `OutstationTask` and return it along with a `DatabaseHandle` for updating it
    pub(crate) fn create(
        link_error_mode: LinkErrorMode,
        config: OutstationConfig,
        event_config: EventBufferConfig,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
    ) -> (Self, OutstationHandle) {
        let (tx, rx) = crate::util::channel::request_channel();
        let handle = DatabaseHandle::new(
            config.max_read_request_headers,
            config.class_zero,
            event_config,
        );
        let (reader, writer) = crate::transport::create_outstation_transport_layer(
            link_error_mode,
            config.outstation_address,
            config.features.self_address,
            config.rx_buffer_size,
        );
        let task = Self {
            session: OutstationSession::new(
                rx,
                config.into(),
                config.into(),
                application,
                information,
                control_handler,
            ),
            reader,
            writer,
            database: handle.clone(),
        };
        (
            task,
            OutstationHandle {
                database: handle,
                sender: tx,
            },
        )
    }

    /// run the outstation task asynchronously until a `SessionError` occurs
    pub(crate) async fn run(&mut self, io: &mut PhysLayer) -> RunError {
        self.session
            .run(io, &mut self.reader, &mut self.writer, &mut self.database)
            .await
    }

    /// process received outstation messages while idle without a session
    pub(crate) async fn process_messages(&mut self) -> Result<(), Shutdown> {
        loop {
            self.session.process_messages().await?;
        }
    }

    pub(crate) fn reset(&mut self) {
        self.session.reset();
        self.reader.reset();
        self.writer.reset();
    }

    #[cfg(test)]
    pub(crate) fn get_reader(&mut self) -> &mut TransportReader {
        &mut self.reader
    }
}
