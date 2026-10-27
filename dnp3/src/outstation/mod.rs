// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

pub use config::*;
#[cfg(feature = "unstable")]
pub use runner::OutstationTask;
pub use traits::*;

use crate::app::Shutdown;
use crate::decode::DecodeLevel;
use crate::outstation::database::{Database, DatabaseHandle};
use crate::outstation::session::task::{ConfigurationChange, OutstationMessage};
use crate::util::channel::Sender;

/// database API to add/remove/update values
pub mod database;

mod config;
/// functionality for processing control requests
pub(crate) mod control;
/// the diagnostics of a READ
pub(crate) mod read_log;
#[cfg(feature = "unstable")]
mod runner;
/// outstation session
pub(crate) mod session;
/// the headers each function supports
pub(crate) mod target;
mod traits;

#[cfg(test)]
mod tests;

/// Handle used to control a running outstation task
#[derive(Clone)]
pub struct OutstationHandle {
    database: DatabaseHandle,
    sender: Sender<OutstationMessage>,
}

impl std::fmt::Debug for OutstationHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutstationHandle").finish_non_exhaustive()
    }
}

impl OutstationHandle {
    /// Get a handle to the associated database
    pub fn get_database_handle(&self) -> DatabaseHandle {
        self.database.clone()
    }

    /// Acquire a mutex on the underlying database and apply a set of changes as a transaction
    pub fn transaction<F, R>(&self, func: F) -> R
    where
        F: FnMut(&mut Database) -> R,
    {
        self.database.transaction(func)
    }

    /// Set the decode level of the outstation
    pub async fn set_decode_level(&mut self, decode_level: DecodeLevel) -> Result<(), Shutdown> {
        self.sender
            .send(ConfigurationChange::SetDecodeLevel(decode_level).into())
            .await?;
        Ok(())
    }

    /// Enable communications
    pub async fn enable(&mut self) -> Result<(), Shutdown> {
        self.sender.send(OutstationMessage::Enable).await?;
        Ok(())
    }

    /// Disable communications
    pub async fn disable(&mut self) -> Result<(), Shutdown> {
        self.sender.send(OutstationMessage::Disable).await?;
        Ok(())
    }

    pub(crate) async fn shutdown(&mut self) -> Result<(), Shutdown> {
        self.sender.send(OutstationMessage::Shutdown).await?;
        Ok(())
    }
}
