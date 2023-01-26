pub use config::*;
pub use traits::*;

use crate::app::Shutdown;
use crate::decode::DecodeLevel;
use crate::outstation::database::{Database, DatabaseHandle};
use crate::outstation::task::{ConfigurationChange, OutstationMessage};
use crate::util::channel::Sender;

/// configuration types

/// database API to add/remove/update values
pub mod database;

mod config;
/// functionality for processing control requests
pub(crate) mod control;
/// handling of deferred read requests
pub(crate) mod deferred;
/// outstation session
pub(crate) mod session;
/// async outstation task
pub(crate) mod task;
mod traits;

#[cfg(test)]
mod tests;

/// Handle used to control a running outstation task
#[derive(Clone)]
pub struct OutstationHandle {
    database: DatabaseHandle,
    sender: Sender<OutstationMessage>,
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
