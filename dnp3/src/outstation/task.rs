use crate::app::parse::DecodeLogLevel;
use crate::outstation::config::*;
use crate::outstation::database::{DatabaseConfig, DatabaseHandle};
use crate::outstation::session::{OutstationSession, SessionError};
use crate::outstation::traits::{ControlHandler, OutstationApplication, OutstationInformation};
use crate::transport::{TransportReader, TransportWriter};
use crate::util::io::IOStream;
use crate::util::task::{Receiver, RunError, Shutdown};

use tracing::Instrument;

pub(crate) enum ConfigurationChange {
    SetDecodeLogLevel(DecodeLogLevel),
}

impl From<ConfigurationChange> for OutstationMessage {
    fn from(x: ConfigurationChange) -> Self {
        OutstationMessage::Configuration(x)
    }
}

#[derive(Debug)]
pub(crate) struct NewSession {
    pub(crate) id: u64,
    pub(crate) io: IOType,
}

impl From<NewSession> for OutstationMessage {
    fn from(x: NewSession) -> Self {
        OutstationMessage::NewSession(x)
    }
}

impl NewSession {
    pub(crate) fn new(id: u64, io: IOType) -> Self {
        Self { id, io }
    }
}

pub(crate) enum OutstationMessage {
    Configuration(ConfigurationChange),
    NewSession(NewSession),
}

#[derive(Debug)]
pub(crate) enum IOType {
    TCPStream(crate::tokio::net::TcpStream),
}

pub struct OutstationTask {
    session: OutstationSession,
    reader: TransportReader,
    writer: TransportWriter,
    database: DatabaseHandle,
}

#[derive(Clone)]
pub struct OutstationHandle {
    pub database: DatabaseHandle,
    sender: crate::tokio::sync::mpsc::Sender<OutstationMessage>,
}

impl OutstationHandle {
    pub async fn set_decode_log_level(&mut self, level: DecodeLogLevel) -> Result<(), Shutdown> {
        self.sender
            .send(ConfigurationChange::SetDecodeLogLevel(level).into())
            .await?;
        Ok(())
    }

    pub(crate) async fn new_io(&mut self, id: u64, io: IOType) -> Result<(), Shutdown> {
        self.sender.send(NewSession::new(id, io).into()).await?;
        Ok(())
    }
}

impl OutstationTask {
    /// create an `OutstationTask` and return it along with a `DatabaseHandle` for updating it
    pub fn create(
        config: OutstationConfig,
        database: DatabaseConfig,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
    ) -> (Self, OutstationHandle) {
        let (tx, rx) = crate::tokio::sync::mpsc::channel(10); // TODO - should this be parameterized?
        let handle = DatabaseHandle::new(database);
        let (reader, writer) = crate::transport::create_outstation_transport_layer(
            config.outstation_address,
            config.features.self_address,
            config.rx_buffer_size,
            config.bubble_framing_errors,
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
    pub(crate) async fn run_io<T>(&mut self, io: &mut T) -> SessionError
    where
        T: IOStream,
    {
        self.session
            .run(io, &mut self.reader, &mut self.writer, &mut self.database)
            .await
    }

    /// run the outstation task asynchronously until a shutdown is requested
    pub(crate) async fn run(&mut self) {
        let mut io = None;

        loop {
            match io.take() {
                None => match self.session.wait_for_io().await {
                    Err(_) => return,
                    Ok(x) => {
                        io.replace(x);
                    }
                },
                Some(session) => {
                    let id = session.id;

                    let result = self
                        .run_one_session(session.io)
                        .instrument(tracing::info_span!("Session", "id" = id))
                        .await;

                    match result {
                        SessionError::Run(RunError::Shutdown) => return,
                        SessionError::Run(RunError::Link(_)) => {
                            // TODO - reset the session
                        }
                        SessionError::NewSession(x) => {
                            io.replace(x);
                        }
                    }
                }
            }
        }
    }

    async fn run_one_session(&mut self, io: IOType) -> SessionError {
        let err = match io {
            IOType::TCPStream(mut stream) => self.run_io(&mut stream).await,
        };
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
