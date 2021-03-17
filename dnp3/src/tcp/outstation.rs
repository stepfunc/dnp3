use tracing::Instrument;

use crate::app::Shutdown;
use crate::link::LinkErrorMode;
use crate::outstation::database::EventBufferConfig;
use crate::outstation::session::RunError;
use crate::outstation::task::OutstationTask;
use crate::outstation::OutstationHandle;
use crate::outstation::*;
use crate::tcp::{AddressFilter, FilterError};
use crate::util::channel::{request_channel, Receiver, Sender};
use crate::util::phys::PhysLayer;

/// message that gets sent to OutstationTaskAdapter when
/// it needs to switch to a new session
#[derive(Debug)]
pub(crate) struct NewSession {
    pub(crate) id: u64,
    pub(crate) phys: PhysLayer,
}

impl NewSession {
    pub(crate) fn new(id: u64, phys: PhysLayer) -> Self {
        Self { id, phys }
    }
}

/// adapts an OutstationTask to something that can listen for new
/// connections on a channel and shutdown an existing session
struct OutstationTaskAdapter {
    receiver: Receiver<NewSession>,
    task: OutstationTask,
}

impl OutstationTaskAdapter {
    fn create(task: OutstationTask) -> (Self, Sender<NewSession>) {
        let (tx, rx) = request_channel();

        (Self { receiver: rx, task }, tx)
    }

    async fn wait_for_session(&mut self) -> Result<NewSession, Shutdown> {
        loop {
            crate::tokio::select! {
                session = self.receiver.receive() => {
                    return session;
                }
                ret = self.task.process_messages() => {
                    ret?
                }
            }
        }
    }

    async fn run_one_session(&mut self, io: &mut PhysLayer) -> Result<NewSession, RunError> {
        crate::tokio::select! {
            res = self.task.run(io) => {
                Err(res)
            }
            x = self.receiver.receive() => {
                Ok(x?)
            }
        }
    }

    async fn run(&mut self) -> Result<(), Shutdown> {
        let mut session = None;

        loop {
            match session.take() {
                None => {
                    session.replace(self.wait_for_session().await?);
                }
                Some(mut s) => {
                    let id = s.id;

                    let result = self
                        .run_one_session(&mut s.phys)
                        .instrument(tracing::info_span!("Session", "id" = id))
                        .await;

                    match result {
                        Ok(new_session) => {
                            // TODO reset the task
                            tracing::warn!(
                                "closing session {} for new session {}",
                                id,
                                new_session.id
                            );
                            // go to next iteration with a new session
                            session.replace(new_session);
                        }
                        Err(RunError::Link(err)) => {
                            // TODO reset the task
                            tracing::warn!("Session error: {}", err);
                        }
                        Err(RunError::Shutdown) => return Err(Shutdown),
                    }
                }
            }
        }
    }
}

struct OutstationInfo {
    filter: AddressFilter,
    handle: OutstationHandle,
    /// how we notify the outstation adapter task to switch to new socket
    sender: Sender<NewSession>,
}

/// A builder for creating a TCP server with one or more outstation instances
/// associated with it
pub struct TcpServer {
    link_error_mode: LinkErrorMode,
    connection_id: u64,
    address: std::net::SocketAddr,
    outstations: Vec<OutstationInfo>,
}

/// Handle to a running server. Dropping the handle, shuts down the server.
pub struct ServerHandle {
    _tx: crate::tokio::sync::oneshot::Sender<()>,
}

impl TcpServer {
    pub fn new(link_error_mode: LinkErrorMode, address: std::net::SocketAddr) -> Self {
        Self {
            link_error_mode,
            connection_id: 0,
            address,
            outstations: Vec::new(),
        }
    }

    pub fn add_outstation(
        &mut self,
        config: OutstationConfig,
        event_config: EventBufferConfig,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
        filter: AddressFilter,
    ) -> Result<(OutstationHandle, impl std::future::Future<Output = ()>), FilterError> {
        for item in self.outstations.iter() {
            if filter.conflicts_with(&item.filter) {
                return Err(FilterError::Conflict);
            }
        }

        let (task, handle) = OutstationTask::create(
            self.link_error_mode,
            config,
            event_config,
            application,
            information,
            control_handler,
        );

        let (mut adapter, tx) = OutstationTaskAdapter::create(task);

        let outstation = OutstationInfo {
            filter,
            handle: handle.clone(),
            sender: tx,
        };
        self.outstations.push(outstation);

        let endpoint = self.address;
        let address = config.outstation_address.raw_value();
        let future = async move {
            let _ = adapter.run()
                .instrument(
                    tracing::info_span!("DNP3-Outstation-TCP", "listen" = ?endpoint, "addr" = address),
                )
                .await;
        };
        Ok((handle, future))
    }

    pub fn spawn_outstation(
        &mut self,
        config: OutstationConfig,
        event_config: EventBufferConfig,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
        filter: AddressFilter,
    ) -> Result<OutstationHandle, FilterError> {
        let (handle, future) = self.add_outstation(
            config,
            event_config,
            application,
            information,
            control_handler,
            filter,
        )?;
        crate::tokio::spawn(future);
        Ok(handle)
    }

    pub async fn bind(
        mut self,
    ) -> Result<(ServerHandle, impl std::future::Future<Output = Shutdown>), crate::tokio::io::Error>
    {
        let listener = crate::tokio::net::TcpListener::bind(self.address).await?;

        let (tx, rx) = crate::tokio::sync::oneshot::channel();

        let task = async move {
            let local = self.address;
            self.run(listener, rx)
                .instrument(tracing::info_span!("TCPServer", "listen" = ?local))
                .await
        };

        let handle = ServerHandle { _tx: tx };

        Ok((handle, task))
    }

    pub async fn bind_and_spawn(self) -> Result<ServerHandle, crate::tokio::io::Error> {
        let (handle, future) = self.bind().await?;
        crate::tokio::spawn(future);
        Ok(handle)
    }

    async fn run(
        &mut self,
        listener: crate::tokio::net::TcpListener,
        rx: crate::tokio::sync::oneshot::Receiver<()>,
    ) -> Shutdown {
        tracing::info!("accepting connections");

        crate::tokio::select! {
             _ = self.accept_loop(listener) => {

             }
             _ = rx => {

             }
        }

        tracing::info!("shutting down outstations");

        for x in self.outstations.iter_mut() {
            // best effort to shutdown outstations before exiting
            let _ = x.handle.shutdown().await;
        }

        tracing::info!("shutdown");

        Shutdown
    }

    async fn accept_loop(
        &mut self,
        mut listener: crate::tokio::net::TcpListener,
    ) -> Result<(), Shutdown> {
        loop {
            self.accept_one(&mut listener).await?;
        }
    }

    async fn accept_one(
        &mut self,
        listener: &mut crate::tokio::net::TcpListener,
    ) -> Result<(), Shutdown> {
        match listener.accept().await {
            Ok((stream, addr)) => {
                self.process_connection(stream, addr).await;
                Ok(())
            }
            Err(err) => {
                tracing::error!("{}", err);
                Err(Shutdown)
            }
        }
    }

    async fn process_connection(
        &mut self,
        stream: crate::tokio::net::TcpStream,
        addr: std::net::SocketAddr,
    ) {
        let id = self.connection_id;
        self.connection_id = self.connection_id.wrapping_add(1);

        tracing::info!("accepted connection {} from: {}", id, addr);

        let first_match = self
            .outstations
            .iter_mut()
            .find(|x| x.filter.matches(addr.ip()));

        match first_match {
            None => {
                tracing::warn!("no matching outstation for: {}", addr)
            }
            Some(x) => {
                let _ = x
                    .sender
                    .send(NewSession::new(
                        id,
                        crate::util::phys::PhysLayer::Tcp(stream),
                    ))
                    .await;
            }
        }
    }
}
