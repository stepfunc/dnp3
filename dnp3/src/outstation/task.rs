use crate::link::error::LinkError;
use crate::outstation::config::*;
use crate::outstation::database::{DatabaseConfig, DatabaseHandle};
use crate::outstation::session::OutstationSession;
use crate::outstation::traits::{ControlHandler, OutstationApplication, OutstationInformation};
use crate::tokio::io::{AsyncRead, AsyncWrite};
use crate::transport::{TransportReader, TransportWriter};

pub struct OutstationTask {
    session: OutstationSession,
    reader: TransportReader,
    writer: TransportWriter,
}

impl OutstationTask {
    /// create an `OutstationTask` and return it along with a `DatabaseHandle` for updating it
    pub fn create(
        config: OutstationConfig,
        database: DatabaseConfig,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
    ) -> (Self, DatabaseHandle) {
        let handle = DatabaseHandle::new(database);
        let (reader, writer) = crate::transport::create_outstation_transport_layer(
            config.outstation_address,
            config.self_address_support,
            config.rx_buffer_size,
        );
        let task = Self {
            session: OutstationSession::new(
                config.into(),
                config.tx_buffer_size,
                handle.clone(),
                application,
                information,
                control_handler,
            ),
            reader,
            writer,
        };
        (task, handle)
    }

    /// run the outstation task asynchronously until a `LinkError` occurs
    pub async fn run<T>(&mut self, io: &mut T) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        self.session
            .run(io, &mut self.reader, &mut self.writer)
            .await
    }

    #[cfg(test)]
    pub(crate) fn get_reader(&mut self) -> &mut TransportReader {
        &mut self.reader
    }
}
