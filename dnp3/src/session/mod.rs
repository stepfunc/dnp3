// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Concrete role composition shared by connection runners.

pub(crate) mod connection;

use crate::app::Shutdown;
use crate::link::EndpointAddress;
use crate::phys::PhysLayer;
use crate::util::deadline::deadline;
use crate::util::lifecycle::{Enabled, RunError, StopReason};
use std::time::Duration;

enum SessionType {
    Master(Box<crate::master::task::MasterTask>),
    Outstation(Box<crate::outstation::session::task::OutstationTask>),
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

    pub(crate) fn outstation(task: crate::outstation::session::task::OutstationTask) -> Self {
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
