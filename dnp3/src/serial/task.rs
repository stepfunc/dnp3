use std::fmt::Display;
use std::time::Duration;

use crate::app::{ExponentialBackOff, Listener, RetryStrategy};
use crate::link::error::LinkError;
use crate::serial::{PortState, SerialSettings};
use crate::util::phys::PhysLayer;
use crate::util::session::{run_connection, Connection, Session};

pub(crate) struct SerialTask {
    path: String,
    serial_settings: SerialSettings,
    back_off: ExponentialBackOff,
    session: Session,
    listener: Box<dyn Listener<PortState>>,
}

impl SerialTask {
    pub(crate) fn new(
        path: &str,
        serial_settings: SerialSettings,
        session: Session,
        retry: RetryStrategy,
        listener: Box<dyn Listener<PortState>>,
    ) -> Self {
        Self {
            path: path.to_string(),
            serial_settings,
            back_off: ExponentialBackOff::new(retry),
            session,
            listener,
        }
    }

    pub(crate) async fn run(&mut self) {
        run_connection(self).await;
        self.listener.update(PortState::Shutdown).get().await;
    }

    async fn wait(&mut self, err: impl Display + Send) -> Duration {
        let delay = self.back_off.on_failure();
        tracing::warn!("{} - waiting {} ms to re-open port", err, delay.as_millis());
        self.listener.update(PortState::Wait(delay)).get().await;
        delay
    }
}

impl Connection for SerialTask {
    type Peer = ();

    fn session(&mut self) -> &mut Session {
        &mut self.session
    }

    async fn disabled(&mut self) {
        self.listener.update(PortState::Disabled).get().await;
    }

    async fn open(&mut self) -> Result<(PhysLayer, ()), Duration> {
        match crate::serial::open(self.path.as_str(), self.serial_settings) {
            Err(err) => Err(self.wait(err).await),
            Ok(serial) => {
                self.back_off.on_success();
                tracing::info!("serial port open");
                self.listener.update(PortState::Open).get().await;
                Ok((PhysLayer::serial(serial), ()))
            }
        }
    }

    async fn failed(&mut self, _: (), err: LinkError) -> Duration {
        // we wait here to prevent any kind of rapid retry scenario if the port opens and then immediately fails
        self.wait(err).await
    }
}
