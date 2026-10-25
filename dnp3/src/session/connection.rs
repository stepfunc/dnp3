// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Connection lifecycle orchestration for concrete sessions.

use std::convert::Infallible;
use std::future::Future;
use std::time::Duration;

use super::Session;
use crate::link::error::LinkError;
use crate::util::lifecycle::{Enabled, RunError, StopReason};
use crate::util::phys::PhysLayer;

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
