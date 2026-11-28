// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::fmt::Display;
use std::time::Duration;

use crate::app::{ExponentialBackOff, Listener, RetryStrategy};
use crate::link::error::LinkError;
use crate::phys::PhysLayer;
use crate::serial::{PortState, SerialSettings};
use crate::session::connection::{run_connection, Connection, OpenError};
use crate::session::Session;

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

    pub(crate) async fn run(mut self) {
        run_connection(&mut self).await;
        let Self {
            session,
            mut listener,
            ..
        } = self;
        session.finish().await;
        listener.update(PortState::Shutdown).get().await;
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

    async fn open(&mut self) -> Result<(PhysLayer, ()), OpenError> {
        // opening is synchronous and can't be interrupted, so the flag is checked around it
        match crate::serial::open(self.path.as_str(), self.serial_settings) {
            // a shutdown requested during the open isn't a failure to report and retry
            Err(_) if self.session.is_shutdown() => Err(OpenError::Shutdown),
            Err(err) => Err(OpenError::Retry(self.wait(err).await)),
            Ok(serial) => {
                self.back_off.on_success();
                tracing::info!("serial port open");
                self.listener.update(PortState::Open).get().await;
                if self.session.is_shutdown() {
                    return Err(OpenError::Shutdown);
                }
                Ok((PhysLayer::serial(serial), ()))
            }
        }
    }

    async fn failed(&mut self, _: (), err: LinkError) -> Duration {
        // we wait here to prevent any kind of rapid retry scenario if the port opens and then immediately fails
        self.wait(err).await
    }
}
