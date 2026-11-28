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
use crate::util::channel::RequestSender;

/// database API to add/remove/update values
pub mod database;

mod config;
/// functionality for processing control requests
pub(crate) mod control;
/// the diagnostics of a READ
pub(crate) mod read_log;
#[cfg(feature = "unstable")]
mod runner;
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
    sender: RequestSender<OutstationMessage>,
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
    ///
    /// `func` runs with the database locked, so it must not wait for space in the outstation's
    /// request queue, as [`OutstationHandle::enable`], [`OutstationHandle::disable`] and
    /// [`OutstationHandle::set_decode_level`] can: if the queue is full while the outstation waits
    /// for the lock, that wait never ends. [`OutstationHandle::shutdown`] never waits.
    pub fn transaction<F, R>(&self, func: F) -> R
    where
        F: FnMut(&mut Database) -> R,
    {
        self.database.transaction(func)
    }

    /// Set the decode level of the outstation
    pub async fn set_decode_level(&self, decode_level: DecodeLevel) -> Result<(), Shutdown> {
        self.sender
            .send(ConfigurationChange::SetDecodeLevel(decode_level).into())
            .await?;
        Ok(())
    }

    /// Enable communications
    pub async fn enable(&self) -> Result<(), Shutdown> {
        self.sender.send(OutstationMessage::Enable).await?;
        Ok(())
    }

    /// Disable communications
    pub async fn disable(&self) -> Result<(), Shutdown> {
        self.sender.send(OutstationMessage::Disable).await?;
        Ok(())
    }

    /// Shut down the outstation task
    ///
    /// The task observes the request at its next wait: for its request queue, a timer, or the
    /// physical layer. A callback in progress completes first, and a request from the master that
    /// overlaps this call can still be answered. Later calls that send to the task fail with
    /// [`Shutdown`]; [`OutstationHandle::transaction`] still updates the database, but nothing
    /// transmits the change. Calling it again, or after the task has ended, does
    /// nothing.
    ///
    /// For an outstation not added to a server, dropping every clone of the handle has the same
    /// effect. A Rust caller that holds a pending
    /// `enable`, `disable` or `set_decode_level` future which has already been given a slot in the queue, and neither polls nor
    /// drops it, delays the task's exit until it does.
    ///
    /// This covers protocol shutdown only. When the Tokio runtime itself shuts down, it drops the
    /// task at its next yield, including inside a user callback that is awaiting.
    ///
    /// An outstation added to a TCP server stops while the server keeps running, and the server
    /// closes later connections for it.
    pub fn shutdown(&self) {
        self.sender.shutdown();
    }

    pub(crate) fn is_shutdown(&self) -> bool {
        self.sender.is_shutdown()
    }

    pub(crate) fn shutdown_listener(&self) -> crate::util::shutdown::ShutdownListener {
        self.sender.listener()
    }
}
