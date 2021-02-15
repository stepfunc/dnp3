use crate::decode::DecodeLevel;
use crate::link::LinkErrorMode;
use crate::outstation::config::*;
use crate::outstation::database::{DatabaseHandle, EventBufferConfig};
use crate::outstation::session::{OutstationSession, SessionError};
use crate::outstation::traits::{ControlHandler, OutstationApplication, OutstationInformation};
use crate::outstation::OutstationHandle;
use crate::transport::{TransportReader, TransportWriter};
use crate::util::phys::PhysLayer;
use crate::util::task::{Receiver, RunError};
use tracing::Instrument;

pub(crate) enum ConfigurationChange {
    SetDecodeLevel(DecodeLevel),
}

impl From<ConfigurationChange> for OutstationMessage {
    fn from(x: ConfigurationChange) -> Self {
        OutstationMessage::Configuration(x)
    }
}

#[derive(Debug)]
pub(crate) struct NewSession {
    pub(crate) id: u64,
    pub(crate) phys: PhysLayer,
}

impl From<NewSession> for OutstationMessage {
    fn from(x: NewSession) -> Self {
        OutstationMessage::ChangeSession(x)
    }
}

impl NewSession {
    pub(crate) fn new(id: u64, phys: PhysLayer) -> Self {
        Self { id, phys }
    }
}

pub(crate) enum OutstationMessage {
    Shutdown,
    Configuration(ConfigurationChange),
    ChangeSession(NewSession),
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
        let (tx, rx) = crate::tokio::sync::mpsc::channel(10); // TODO - should this be parameterized?
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
                Receiver::new(rx),
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
    pub(crate) async fn run_io(&mut self, io: &mut PhysLayer) -> SessionError {
        self.session
            .run(io, &mut self.reader, &mut self.writer, &mut self.database)
            .await
    }

    /// run the outstation task asynchronously until a shutdown is requested
    pub(crate) async fn run(&mut self) {
        let mut session = None;

        loop {
            match session.take() {
                None => match self.session.wait_for_io().await {
                    Err(_) => return,
                    Ok(s) => {
                        session.replace(s);
                    }
                },
                Some(s) => {
                    let id = s.id;

                    let result = self
                        .run_one_session(s.phys)
                        .instrument(tracing::info_span!("Session", "id" = id))
                        .await;

                    match result {
                        SessionError::Run(RunError::Shutdown) => return,
                        SessionError::Run(RunError::Link(_)) => {
                            // TODO - reset the session
                        }
                        SessionError::NewSession(s) => {
                            session.replace(s);
                        }
                    }
                }
            }
        }
    }

    async fn run_one_session(&mut self, mut phys: PhysLayer) -> SessionError {
        let err = self.run_io(&mut phys).await;
        match &err {
            SessionError::Run(RunError::Shutdown) => {
                tracing::info!("received shutdown");
            }
            SessionError::Run(RunError::Link(err)) => {
                tracing::warn!("link error: {}", err);
            }
            SessionError::NewSession(session) => {
                tracing::info!("closing for new connection: {}", session.id)
            }
        };
        err
    }

    #[cfg(test)]
    pub(crate) fn get_reader(&mut self) -> &mut TransportReader {
        &mut self.reader
    }
}
