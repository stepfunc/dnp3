use crate::link::error::LinkError;

/// Indicates that a task shutdown has been requested
#[derive(Copy, Clone, Debug)]
pub struct Shutdown;

pub(crate) struct Receiver<T> {
    inner: crate::tokio::sync::mpsc::Receiver<T>,
}

impl<T> Receiver<T> {
    pub(crate) fn new(inner: crate::tokio::sync::mpsc::Receiver<T>) -> Self {
        Receiver { inner }
    }

    pub(crate) async fn next(&mut self) -> Result<T, Shutdown> {
        match self.inner.recv().await {
            Some(x) => Ok(x),
            None => Err(Shutdown),
        }
    }
}

impl From<crate::tokio::sync::oneshot::error::RecvError> for Shutdown {
    fn from(_: crate::tokio::sync::oneshot::error::RecvError) -> Self {
        Shutdown
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum RunError {
    Link(LinkError),
    Shutdown,
}

impl From<LinkError> for RunError {
    fn from(err: LinkError) -> Self {
        RunError::Link(err)
    }
}

impl From<Shutdown> for RunError {
    fn from(_: Shutdown) -> Self {
        RunError::Shutdown
    }
}

impl std::fmt::Display for Shutdown {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("request could not be completed because the component has shut down")
    }
}

impl std::error::Error for Shutdown {}
