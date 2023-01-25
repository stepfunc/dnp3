use crate::app::{Listener, Shutdown};
use crate::serial::{PortState, SerialSettings};
use crate::shared::{RunError, Session, StopReason};
use crate::util::phys::PhysLayer;
use std::time::Duration;

pub(crate) struct SerialTask {
    path: String,
    serial_settings: SerialSettings,
    retry_delay: Duration,
    session: Session,
    listener: Box<dyn Listener<PortState>>,
}

impl SerialTask {
    pub(crate) fn new(
        path: &str,
        serial_settings: SerialSettings,
        session: Session,
        retry_delay: Duration,
        listener: Box<dyn Listener<PortState>>,
    ) -> Self {
        Self {
            path: path.to_string(),
            serial_settings,
            retry_delay,
            session,
            listener,
        }
    }

    pub(crate) async fn run(&mut self) {
        let _ = self.run_impl().await;
        self.session.shutdown().await;
        self.listener.update(PortState::Shutdown).get().await;
    }

    async fn run_impl(&mut self) -> Result<(), Shutdown> {
        loop {
            self.listener.update(PortState::Disabled).get().await;
            self.session.wait_for_enabled().await?;
            if let Err(StopReason::Shutdown) = self.run_enabled().await {
                return Err(Shutdown);
            }
        }
    }

    async fn run_enabled(&mut self) -> Result<(), StopReason> {
        loop {
            match crate::serial::open(self.path.as_str(), self.serial_settings) {
                Err(err) => {
                    tracing::warn!(
                        "{} - waiting {} ms to re-open port",
                        err,
                        self.retry_delay.as_millis()
                    );
                    self.listener
                        .update(PortState::Wait(self.retry_delay))
                        .get()
                        .await;
                    self.session.wait_for_retry(self.retry_delay).await?;
                }
                Ok(serial) => {
                    let mut io = PhysLayer::Serial(serial);
                    tracing::info!("serial port open");
                    self.listener.update(PortState::Open).get().await;
                    match self.session.run(&mut io).await {
                        RunError::Stop(x) => {
                            return Err(x);
                        }
                        RunError::Link(err) => {
                            tracing::warn!("serial port error: {}", err);
                            tracing::info!(
                                "waiting {} ms to re-open",
                                self.retry_delay.as_millis()
                            );
                            self.listener
                                .update(PortState::Wait(self.retry_delay))
                                .get()
                                .await;
                            self.session.wait_for_retry(self.retry_delay).await?;
                        }
                    }
                }
            }
        }
    }
}
