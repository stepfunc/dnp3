use crate::app::Shutdown;
use crate::link::error::LinkError;

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
