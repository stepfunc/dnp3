use crate::app::parse::DecodeLogLevel;
use crate::outstation::config::*;
use crate::outstation::database::{DatabaseConfig, DatabaseHandle};
use crate::outstation::session::{OutstationSession, SessionError};
use crate::outstation::traits::{ControlHandler, OutstationApplication, OutstationInformation};
use crate::transport::{TransportReader, TransportWriter};
use crate::util::io::IOStream;
use crate::util::task::{Receiver, Shutdown};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum OutstationMessage {
    SetDecodeLogLevel(DecodeLogLevel),
}

pub struct OutstationTask {
    session: OutstationSession,
    reader: TransportReader,
    writer: TransportWriter,
    database: DatabaseHandle,
}

pub struct OutstationHandle {
    pub database: DatabaseHandle,
    sender: crate::tokio::sync::mpsc::Sender<OutstationMessage>,
}

impl OutstationHandle {
    pub async fn set_decode_log_level(&mut self, level: DecodeLogLevel) -> Result<(), Shutdown> {
        self.sender
            .send(OutstationMessage::SetDecodeLogLevel(level))
            .await?;
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
    pub async fn run<T>(&mut self, io: &mut T) -> Result<(), SessionError>
    where
        T: IOStream,
    {
        self.session
            .run(io, &mut self.reader, &mut self.writer, &mut self.database)
            .await
    }

    #[cfg(test)]
    pub(crate) fn get_reader(&mut self) -> &mut TransportReader {
        &mut self.reader
    }
}
