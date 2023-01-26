use tracing::Instrument;

use crate::app::{Listener, Shutdown};
use crate::outstation::ConnectionState;
use crate::util::channel::{request_channel, Receiver, Sender};
use crate::util::phys::PhysLayer;
use crate::util::session::{Enabled, RunError, Session, StopReason};

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

/// Wraps a session allowing it to receive new physical layers from a server component
pub(crate) struct ServerTask {
    receiver: Receiver<NewSession>,
    session: Session,
    listener: Box<dyn Listener<ConnectionState>>,
}

impl ServerTask {
    pub(crate) fn create(
        session: Session,
        listener: Box<dyn Listener<ConnectionState>>,
    ) -> (Self, Sender<NewSession>) {
        let (tx, rx) = request_channel();
        (
            Self {
                receiver: rx,
                session,
                listener,
            },
            tx,
        )
    }

    async fn wait_for_session(&mut self) -> Result<NewSession, Shutdown> {
        loop {
            tokio::select! {
                session = self.receiver.receive() => {
                    return session;
                }
                ret = self.session.process_next_message() => {
                    if let Err(StopReason::Shutdown) = ret {
                        return Err(Shutdown);
                    }
                }
            }
        }
    }

    async fn run_one_session(&mut self, io: &mut PhysLayer) -> Result<NewSession, RunError> {
        tokio::select! {
            res = self.session.run(io) => {
                Err(res)
            }
            x = self.receiver.receive() => {
                Ok(x?)
            }
        }
    }

    pub(crate) async fn run_new_session(
        &mut self,
        mut session: NewSession,
    ) -> Result<Option<NewSession>, Shutdown> {
        let id = session.id;
        self.listener.update(ConnectionState::Connected).get().await;
        let result = self
            .run_one_session(&mut session.phys)
            .instrument(tracing::info_span!("session", "id" = id))
            .await;
        self.listener
            .update(ConnectionState::Disconnected)
            .get()
            .await;

        match result {
            Ok(new_session) => {
                tracing::warn!("closing session {} for new session {}", id, new_session.id);
                // go to next iteration with a new session
                Ok(Some(new_session))
            }
            Err(RunError::Link(err)) => {
                // go to next iteration to get a new session
                tracing::warn!("session error: {}", err);
                Ok(None)
            }
            Err(RunError::Stop(StopReason::Shutdown)) => Err(Shutdown),
            Err(RunError::Stop(StopReason::Disable)) => Ok(None),
        }
    }

    pub(crate) async fn run(&mut self) -> Result<(), Shutdown> {
        let mut session = None;

        loop {
            match session.take() {
                None => {
                    session.replace(self.wait_for_session().await?);
                }
                Some(s) => {
                    if self.session.enabled() == Enabled::Yes {
                        if let Some(s) = self.run_new_session(s).await? {
                            session.replace(s);
                        }
                    } else {
                        tracing::warn!("Ignoring new connection while disabled: {}", s.id);
                    }
                }
            }
        }
    }
}
