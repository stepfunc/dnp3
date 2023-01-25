use crate::app::Shutdown;
use crate::link::error::LinkError;
use crate::util::phys::PhysLayer;
use std::time::Duration;

/// Communication sessions might be stopped due to being disabled, or due to being shutdown permanently
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum StopReason {
    /// Communication channel is temporarily disabled
    Disable,
    /// Communication channel is permanently shut down
    Shutdown,
}

/// Communication sessions might terminate due to being explicitly stopped or because their was some I/O error
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum RunError {
    /// Stopped by the user
    Stop(StopReason),
    /// Error occurred due to underlying I/O or bad link data
    Link(LinkError),
}

impl From<Shutdown> for StopReason {
    fn from(_: Shutdown) -> Self {
        StopReason::Shutdown
    }
}

impl From<StopReason> for RunError {
    fn from(x: StopReason) -> Self {
        RunError::Stop(x)
    }
}

impl From<LinkError> for RunError {
    fn from(err: LinkError) -> Self {
        RunError::Link(err)
    }
}

impl From<Shutdown> for RunError {
    fn from(_: Shutdown) -> Self {
        RunError::Stop(StopReason::Shutdown)
    }
}

enum SessionType {
    Master(crate::master::task::MasterTask),
}

/// Wrapper around a specific type of session (master or outstation)
///
/// Allows for the sharing of transport connectivity code (e.g. TCP/TLS client + serial)
/// between masters and outstations
pub(crate) struct Session {
    inner: SessionType,
}

impl Session {
    pub(crate) fn master(task: crate::master::task::MasterTask) -> Self {
        Self {
            inner: SessionType::Master(task),
        }
    }

    fn is_enabled(&self) -> bool {
        match &self.inner {
            SessionType::Master(x) => x.is_enabled(),
        }
    }

    async fn process_next_message(&mut self) -> Result<(), StopReason> {
        match &mut self.inner {
            SessionType::Master(x) => x.process_next_message().await,
        }
    }

    pub(crate) async fn run(&mut self, io: &mut PhysLayer) -> RunError {
        match &mut self.inner {
            SessionType::Master(x) => x.run(io).await,
        }
    }

    pub(crate) async fn shutdown(&mut self) {
        match &mut self.inner {
            SessionType::Master(x) => x.shutdown().await,
        }
    }

    pub(crate) async fn wait_for_enabled(&mut self) -> Result<(), Shutdown> {
        loop {
            if self.is_enabled() {
                return Ok(());
            }

            if let Err(StopReason::Shutdown) = self.process_next_message().await {
                return Err(Shutdown);
            }
        }
    }

    pub(crate) async fn wait_for_retry(&mut self, duration: Duration) -> Result<(), StopReason> {
        use std::ops::Add;

        let deadline = tokio::time::Instant::now().add(duration);

        loop {
            tokio::select! {
                result = self.process_next_message() => {
                   result?;
                   if !self.is_enabled() {
                       return Err(StopReason::Disable)
                   }
                }
                _ = tokio::time::sleep_until(deadline) => {
                   return Ok(());
                }
            }
        }
    }
}