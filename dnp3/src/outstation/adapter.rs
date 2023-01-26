use tracing::Instrument;

use crate::app::{Listener, Shutdown};
use crate::outstation::task::OutstationTask;
use crate::outstation::ConnectionState;
use crate::util::channel::{request_channel, Receiver, Sender};
use crate::util::phys::PhysLayer;
use crate::util::session::{Enabled, RunError, StopReason};

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
pub(crate) struct OutstationTaskAdapter {
    receiver: Receiver<NewSession>,
    task: OutstationTask,
    listener: Box<dyn Listener<ConnectionState>>,
}

impl OutstationTaskAdapter {
    pub(crate) fn create(
        task: OutstationTask,
        listener: Box<dyn Listener<ConnectionState>>,
    ) -> (Self, Sender<NewSession>) {
        let (tx, rx) = request_channel();
        (
            Self {
                receiver: rx,
                task,
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
                ret = self.task.process_messages() => {
                    if let Err(StopReason::Shutdown) = ret {
                        return Err(Shutdown);
                    }
                }
            }
        }
    }

    async fn run_one_session(&mut self, io: &mut PhysLayer) -> Result<NewSession, RunError> {
        tokio::select! {
            res = self.task.run(io) => {
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

        // reset outstation state in between sessions
        self.task.reset();

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
            Err(RunError::Stop(StopReason::Shutdown)) => return Err(Shutdown),
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
                    if self.task.enabled() == Enabled::Yes {
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
