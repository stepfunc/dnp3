use crate::tokio::io::{AsyncRead, AsyncWrite};

use crate::app::retry::ExponentialBackOff;
use crate::master::error::Shutdown;
use crate::master::handle::{Listener, MasterConfiguration, MasterHandle};
use crate::master::session::{MasterSession, RunError};
use crate::transport::TransportReader;
use crate::transport::TransportWriter;
use std::future::Future;
use std::io::Error;
use std::time::Duration;

/// entry points for creating and spawning serial-based master tasks
pub mod serial;
/// entry points for creating and spawning TCP-based master tasks
pub mod tcp;

/// state of TCP client connection
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ClientState {
    Connecting,
    Connected,
    WaitAfterFailedConnect(Duration),
    WaitAfterDisconnect(Duration),
    Shutdown,
}

pub(crate) struct MasterTask<S, Fut, F>
where
    S: AsyncRead + AsyncWrite + Unpin,
    Fut: Future<Output = Result<S, Error>>,
    F: Fn() -> Fut,
{
    connect_fn: F,
    back_off: ExponentialBackOff,
    session: MasterSession,
    reader: TransportReader,
    writer: TransportWriter,
    listener: Listener<ClientState>,
}

impl<S, Fut, F> MasterTask<S, Fut, F>
where
    S: AsyncRead + AsyncWrite + Unpin,
    Fut: Future<Output = Result<S, Error>>,
    F: Fn() -> Fut,
{
    fn new(
        connect_fn: F,
        config: MasterConfiguration,
        listener: Listener<ClientState>,
    ) -> (Self, MasterHandle) {
        let (tx, rx) = crate::tokio::sync::mpsc::channel(100); // TODO
        let session = MasterSession::new(
            config.level,
            config.response_timeout,
            config.tx_buffer_size,
            rx,
        );
        let (reader, writer) =
            crate::transport::create_master_transport_layer(config.address, config.rx_buffer_size);
        let task = Self {
            connect_fn,
            back_off: ExponentialBackOff::new(config.reconnection_strategy),
            session,
            reader,
            writer,
            listener,
        };
        (task, MasterHandle::new(tx))
    }

    async fn run(&mut self) {
        self.run_impl().await.ok();
    }

    async fn run_impl(&mut self) -> Result<(), Shutdown> {
        loop {
            self.listener.update(ClientState::Connecting);
            match (self.connect_fn)().await {
                Err(err) => {
                    let delay = self.back_off.on_failure();
                    log::warn!("{} - waiting {} ms to retry", err, delay.as_millis());
                    self.listener
                        .update(ClientState::WaitAfterFailedConnect(delay));
                    self.session.delay_for(delay).await?;
                }
                Ok(mut socket) => {
                    log::info!("connected");
                    self.back_off.on_success();
                    self.listener.update(ClientState::Connected);
                    match self
                        .session
                        .run(&mut socket, &mut self.writer, &mut self.reader)
                        .await
                    {
                        RunError::Shutdown => {
                            self.listener.update(ClientState::Shutdown);
                            return Err(Shutdown);
                        }
                        RunError::Link(err) => {
                            let delay = self.back_off.min_delay();
                            log::warn!("{} - waiting {} ms to reconnect", err, delay.as_millis());
                            self.listener
                                .update(ClientState::WaitAfterDisconnect(delay));
                            self.session.delay_for(delay).await?;
                        }
                    }
                }
            }
        }
    }
}
