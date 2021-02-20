use crate::app::Shutdown;

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
