use crate::app::parse::options::ParseOptions;
use crate::decode::DecodeLevel;
use crate::link::reader::LinkModes;
use crate::link::EndpointAddress;
use crate::outstation::config::*;
use crate::outstation::database::DatabaseHandle;
use crate::outstation::session::OutstationSession;
use crate::outstation::traits::{ControlHandler, OutstationApplication, OutstationInformation};
use crate::outstation::OutstationHandle;
use crate::transport::{FragmentAddr, RealTransport, Transport, TransportReader, TransportWriter};
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

pub(crate) struct OutstationTask<T: Transport = RealTransport> {
    session: OutstationSession,
    reader: TransportReader<T>,
    writer: TransportWriter<T>,
    database: DatabaseHandle,
}

/// RAII guard that re-arms per-connection state on session teardown.
struct SessionTeardown<'a, T: Transport> {
    task: &'a mut OutstationTask<T>,
}

impl<T: Transport> Drop for SessionTeardown<'_, T> {
    fn drop(&mut self) {
        // re-arm, don't clear: unconfirmed events return to Unselected for re-delivery
        self.task.database.reset();
        self.task.reader.reset();
        self.task.writer.reset();
        self.task.session.reset_session_state();
    }
}

impl OutstationTask {
    /// create an `OutstationTask` and return it along with an `OutstationHandle` for controlling it
    #[expect(
        clippy::too_many_arguments,
        reason = "a task takes each part of its configuration separately"
    )]
    pub(crate) fn create(
        initial_state: Enabled,
        link_modes: LinkModes,
        parse_options: ParseOptions,
        config: OutstationConfig,
        phys_addr: PhysAddr,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
    ) -> (Self, OutstationHandle) {
        Self::with_transport(
            initial_state,
            link_modes,
            parse_options,
            config,
            phys_addr,
            application,
            information,
            control_handler,
        )
    }
}

#[cfg(test)]
impl OutstationTask<crate::transport::mock::MockTransport> {
    pub(crate) fn set_rx_frame_info(&mut self, info: crate::link::header::FrameInfo) {
        self.reader.set_rx_frame_info(info);
    }

    pub(crate) fn rx_frame_info(&self) -> crate::transport::mock::reader::RxFrameInfo {
        self.reader.rx_frame_info()
    }
}

impl<T: Transport> OutstationTask<T> {
    /// create an `OutstationTask` over a transport and return it along with an `OutstationHandle`
    /// for controlling it
    #[expect(
        clippy::too_many_arguments,
        reason = "a task takes each part of its configuration separately"
    )]
    pub(crate) fn with_transport(
        initial_state: Enabled,
        link_modes: LinkModes,
        parse_options: ParseOptions,
        config: OutstationConfig,
        phys_addr: PhysAddr,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
    ) -> (Self, OutstationHandle) {
        let (tx, rx) = crate::util::channel::request_channel();
        let handle = DatabaseHandle::new(
            config.read_header_limit(),
            config.class_zero,
            config.event_buffer_config,
        );
        let (reader, writer) = crate::transport::create_transport_layer(
            link_modes,
            parse_options,
            crate::link::LinkRole::Outstation {
                self_address: config.features.self_address,
            },
            config.outstation_address,
            config.rx_buffer_size.value(),
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

    pub(crate) fn change_master_address(&mut self, address: EndpointAddress) {
        self.session.change_master_address(address);
    }

    /// run the outstation task asynchronously until a `RunError` occurs
    ///
    /// Cancel-safe: per-connection state is re-armed by `SessionTeardown`'s `Drop`, so
    /// dropping this future tears down exactly as returning from it does.
    pub(crate) async fn run(&mut self, io: &mut PhysLayer) -> RunError {
        let guard = SessionTeardown { task: self };
        let task = &mut *guard.task; // reborrow so Drop can reset after the run future releases its borrows
        task.session
            .run(io, &mut task.reader, &mut task.writer, &mut task.database)
            .await
    }

    /// process received outstation messages while idle without a session
    pub(crate) async fn process_next_message(&mut self) -> Result<(), StopReason> {
        self.session.process_next_message().await
    }
}
