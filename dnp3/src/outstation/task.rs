use crate::app::parse::DecodeLogLevel;
use crate::outstation::config::*;
use crate::outstation::database::{DatabaseConfig, DatabaseHandle};
use crate::outstation::session::{OutstationSession, SessionError};
use crate::outstation::traits::{ControlHandler, OutstationApplication, OutstationInformation};
use crate::transport::{TransportReader, TransportWriter};
use crate::util::io::IOStream;
use crate::util::task::{Receiver, RunError, Shutdown};
use std::collections::VecDeque;

pub(crate) enum ConfigurationChange {
    SetDecodeLogLevel(DecodeLogLevel),
}

impl From<ConfigurationChange> for OutstationMessage {
    fn from(x: ConfigurationChange) -> Self {
        OutstationMessage::Configuration(x)
    }
}

pub(crate) enum OutstationMessage {
    Configuration(ConfigurationChange),
    NewSession(IOType),
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

    pub(crate) async fn new_io(&mut self, io: IOType) -> Result<(), Shutdown> {
        self.sender.send(OutstationMessage::NewSession(io)).await?;
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
        let mut vec = VecDeque::with_capacity(1);

        loop {
            match vec.pop_front() {
                None => match self.session.wait_for_io().await {
                    Err(_) => return,
                    Ok(io) => vec.push_back(io),
                },
                Some(io) => {
                    match self.run_one_session(io).await {
                        SessionError::Run(RunError::Shutdown) => return,
                        SessionError::Run(RunError::Link(err)) => {
                            // TODO - reset the session
                            tracing::warn!("{}", err);
                        }
                        SessionError::NewSession(io) => {
                            println!("new io!");
                            tracing::info!("session closed - new connection");
                            vec.push_back(io);
                        }
                    }
                }
            }
        }
    }

    async fn run_one_session(&mut self, io: IOType) -> SessionError {
        match io {
            IOType::TCPStream(mut stream) => self.run_io(&mut stream).await,
        }
    }

    #[cfg(test)]
    pub(crate) fn get_reader(&mut self) -> &mut TransportReader {
        &mut self.reader
    }
}
