// use crate::app::parse::parser::ParseLogLevel;
//use tokio::prelude::{AsyncRead, AsyncWrite};

/*
/// Configuration of a master task
pub struct MasterTask {
    _runner: RequestRunner,
}

#[derive(Copy, Clone)]
pub struct Configuration {
    level: ParseLogLevel,
    response_timeout: std::time::Duration,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SessionError {
    /// the stream errored, and must be re-established
    IOError,
    /// unrecoverable framing issue occurred, stream must be re-established
    BadFrame,
    /// a shutdown was requested
    Shutdown,
}

impl MasterTask {
    pub fn new(config: Configuration) -> Self {
        Self {
            _runner: RequestRunner::new(config.level, config.response_timeout),
        }
    }

    pub async fn run<T>(&mut self, mut _io: T) -> SessionError
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        SessionError::Shutdown
    }
}
*/
