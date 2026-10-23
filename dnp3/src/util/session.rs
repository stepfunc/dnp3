// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::Shutdown;
use crate::link::error::LinkError;
use crate::link::EndpointAddress;
use crate::util::deadline::deadline;
use crate::util::phys::PhysLayer;
use std::convert::Infallible;
use std::future::Future;
use std::time::Duration;

/// Just used to avoid bool in methods
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Enabled {
    /// Communication session is enabled
    Yes,
    /// Communication session is disabled
    No,
}

/// Communication sessions might be stopped due to being disabled, or due to being shutdown permanently
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum StopReason {
    /// Communication channel is temporarily disabled
    Disable,
    /// Communication channel is permanently shut down
    Shutdown,
}

impl std::fmt::Display for StopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            StopReason::Disable => write!(f, "disabled"),
            StopReason::Shutdown => write!(f, "shutdown"),
        }
    }
}

/// Communication sessions might terminate due to being explicitly stopped or because their was some I/O error
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum RunError {
    /// Stopped by the user
    Stop(StopReason),
    /// Error occurred due to underlying I/O or bad link data
    Link(LinkError),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RunError::Stop(err) => write!(f, "{err}"),
            RunError::Link(err) => write!(f, "{err}"),
        }
    }
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
    Master(Box<crate::master::task::MasterTask>),
    Outstation(Box<crate::outstation::task::OutstationTask>),
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
            inner: SessionType::Master(Box::new(task)),
        }
    }

    pub(crate) fn outstation(task: crate::outstation::task::OutstationTask) -> Self {
        Self {
            inner: SessionType::Outstation(Box::new(task)),
        }
    }

    pub(crate) fn change_master_address(&mut self, address: EndpointAddress) {
        match &mut self.inner {
            SessionType::Master(_) => {
                tracing::warn!("Attempted to change master address on a master session. This feature is only supported for outstation clients.");
            }
            SessionType::Outstation(x) => {
                x.change_master_address(address);
            }
        }
    }

    pub(crate) fn enabled(&self) -> Enabled {
        match &self.inner {
            SessionType::Master(x) => x.enabled(),
            SessionType::Outstation(x) => x.enabled(),
        }
    }

    pub(crate) async fn process_next_message(&mut self) -> Result<(), StopReason> {
        match &mut self.inner {
            SessionType::Master(x) => x.process_next_message().await,
            SessionType::Outstation(x) => x.process_next_message().await,
        }
    }

    pub(crate) async fn run(&mut self, io: &mut PhysLayer) -> RunError {
        match &mut self.inner {
            SessionType::Master(x) => x.run(io).await,
            SessionType::Outstation(x) => x.run(io).await,
        }
    }

    pub(crate) async fn wait_for_enabled(&mut self) -> Result<(), Shutdown> {
        loop {
            if self.enabled() == Enabled::Yes {
                return Ok(());
            }

            if let Err(StopReason::Shutdown) = self.process_next_message().await {
                return Err(Shutdown);
            }
        }
    }

    pub(crate) async fn wait_for_retry(&mut self, duration: Duration) -> Result<(), StopReason> {
        let deadline = deadline(tokio::time::Instant::now(), duration);

        loop {
            tokio::select! {
                result = self.process_next_message() => {
                   result?;
                   if self.enabled() == Enabled::No {
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

/// The transport-specific parts of a channel that opens a physical layer, runs a [`Session`] on
/// it, and retries when either fails
///
/// Each method reports its event to the channel's listener, if it has one.
pub(crate) trait Connection: Send {
    /// What the channel remembers about an open physical layer until it closes
    type Peer: Send;

    /// Keep the physical layer open until the wait after a failed session ends
    const CLOSE_AFTER_RETRY_WAIT: bool = true;

    fn session(&mut self) -> &mut Session;

    /// The session is disabled
    fn disabled(&mut self) -> impl Future<Output = ()> + Send;

    /// Open the physical layer, or return the delay before trying again
    fn open(&mut self) -> impl Future<Output = Result<(PhysLayer, Self::Peer), Duration>> + Send;

    /// The session failed, returning the delay before opening again, which can be zero
    fn failed(&mut self, peer: Self::Peer, err: LinkError)
        -> impl Future<Output = Duration> + Send;
}

/// Run a channel until it's shut down: wait while it's disabled, open the physical layer, run the
/// session on it, and wait before retrying when either fails
pub(crate) async fn run_connection<C: Connection>(connection: &mut C) {
    while let Err(StopReason::Disable) = run_enabled(connection).await {}
}

/// Wait until the session is enabled, then run it until it's disabled or shut down
async fn run_enabled<C: Connection>(connection: &mut C) -> Result<Infallible, StopReason> {
    if connection.session().enabled() == Enabled::No {
        connection.disabled().await;
        connection.session().wait_for_enabled().await?;
    }
    loop {
        let (mut phys, peer) = match connection.open().await {
            Ok(x) => x,
            Err(delay) => {
                connection.session().wait_for_retry(delay).await?;
                continue;
            }
        };
        let err = match connection.session().run(&mut phys).await {
            RunError::Stop(reason) => return Err(reason),
            RunError::Link(err) => err,
        };
        let delay = connection.failed(peer, err).await;
        // dropped here, or after the wait
        let _phys = C::CLOSE_AFTER_RETRY_WAIT.then_some(phys);
        if !delay.is_zero() {
            connection.session().wait_for_retry(delay).await?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::reader::LinkModes;
    use crate::master::task::MasterTask;
    use crate::master::{MasterChannel, MasterChannelConfig, MasterChannelType};

    #[tokio::test]
    async fn retry_wait_accepts_a_delay_too_long_to_represent() {
        let (tx, rx) = crate::util::channel::request_channel();
        let mut session = Session::master(MasterTask::new(
            Enabled::Yes,
            LinkModes::test(),
            MasterChannelConfig::new(EndpointAddress::raw(1)),
            rx,
        ));
        let mut channel = MasterChannel::new(tx, MasterChannelType::Stream);

        let (result, _) = tokio::join!(session.wait_for_retry(Duration::MAX), channel.disable());
        assert!(matches!(result, Err(StopReason::Disable)));
    }
}
