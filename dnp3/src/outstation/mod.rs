pub use config::*;
pub use traits::*;

use crate::app::Shutdown;
use crate::decode::DecodeLevel;
use crate::outstation::database::DatabaseHandle;
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

#[derive(Clone)]
pub struct OutstationHandle {
    pub database: DatabaseHandle,
    sender: Sender<OutstationMessage>,
}

impl OutstationHandle {
    pub async fn set_decode_level(&mut self, decode_level: DecodeLevel) -> Result<(), Shutdown> {
        self.sender
            .send(ConfigurationChange::SetDecodeLevel(decode_level).into())
            .await?;
        Ok(())
    }

    pub(crate) async fn shutdown(&mut self) -> Result<(), Shutdown> {
        self.sender.send(OutstationMessage::Shutdown).await?;
        Ok(())
    }
}
