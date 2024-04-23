use crate::decode::DecodeLevel;
use crate::link::reader::LinkModes;
use crate::outstation::config::*;
use crate::outstation::database::DatabaseHandle;
use crate::outstation::session::OutstationSession;
use crate::outstation::traits::{ControlHandler, OutstationApplication, OutstationInformation};
use crate::outstation::OutstationHandle;
use crate::transport::{FragmentAddr, TransportReader, TransportWriter};
use crate::util::phys::{PhysAddr, PhysLayer};
use crate::util::session::{Enabled, RunError, StopReason};

pub(crate) enum ConfigurationChange {
    SetDecodeLevel(DecodeLevel),
}

impl From<ConfigurationChange> for OutstationMessage {
    fn from(x: ConfigurationChange) -> Self {
        OutstationMessage::Configuration(x)
    }
}

pub(crate) enum OutstationMessage {
    Enable,
    Disable,
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
        initial_state: Enabled,
        link_modes: LinkModes,
        config: OutstationConfig,
        phys_addr: PhysAddr,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
    ) -> (Self, OutstationHandle) {
        let (tx, rx) = crate::util::channel::request_channel();
        let handle = DatabaseHandle::new(
            config.max_read_request_headers,
            config.class_zero,
            config.event_buffer_config,
        );
        let (reader, writer) = crate::transport::create_outstation_transport_layer(
            link_modes,
            config.outstation_address,
            config.features.self_address,
            config.rx_buffer_size,
        );
        let destination = FragmentAddr {
            link: config.master_address,
            phys: phys_addr,
        };
        let task = Self {
            session: OutstationSession::new(
                initial_state,
                rx,
                destination,
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

    pub(crate) fn enabled(&self) -> Enabled {
        self.session.enabled()
    }

    /// run the outstation task asynchronously until a `SessionError` occurs
    pub(crate) async fn run(&mut self, io: &mut PhysLayer) -> RunError {
        let res = self
            .session
            .run(io, &mut self.reader, &mut self.writer, &mut self.database)
            .await;

        self.reader.reset();
        self.writer.reset();

        res
    }

    /// process received outstation messages while idle without a session
    pub(crate) async fn process_next_message(&mut self) -> Result<(), StopReason> {
        self.session.process_next_message().await
    }

    #[cfg(test)]
    pub(crate) fn get_reader(&mut self) -> &mut TransportReader {
        &mut self.reader
    }
}
