/// configuration types

/// database API to add/remove/update values
pub mod database;

mod config;
mod traits;
pub use config::*;
pub use traits::*;

/// functionality for processing control requests
pub(crate) mod control;
/// handling of deferred read requests
pub(crate) mod deferred;
/// outstation session
pub(crate) mod session;
/// async outstation task
pub(crate) mod task;

#[cfg(test)]
mod tests;

use crate::decode::DecodeLevel;
use crate::outstation::database::DatabaseHandle;
use crate::outstation::task::{ConfigurationChange, NewSession, OutstationMessage};
use crate::util::phys::PhysLayer;
use crate::util::task::Shutdown;

#[derive(Clone)]
pub struct OutstationHandle {
    pub database: DatabaseHandle,
    sender: crate::tokio::sync::mpsc::Sender<OutstationMessage>,
}

impl OutstationHandle {
    pub async fn set_decode_level(&mut self, decode_level: DecodeLevel) -> Result<(), Shutdown> {
        self.sender
            .send(ConfigurationChange::SetDecodeLevel(decode_level).into())
            .await?;
        Ok(())
    }

    pub(crate) async fn change_session(
        &mut self,
        id: u64,
        phys: PhysLayer,
    ) -> Result<(), Shutdown> {
        self.sender
            .send(OutstationMessage::ChangeSession(NewSession::new(id, phys)))
            .await?;
        Ok(())
    }

    pub(crate) async fn shutdown(&mut self) -> Result<(), Shutdown> {
        self.sender.send(OutstationMessage::Shutdown).await?;
        Ok(())
    }
}
