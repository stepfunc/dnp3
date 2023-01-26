use crate::app::{ExponentialBackOff, Listener, RetryStrategy, Shutdown};
use crate::serial::{PortState, SerialSettings};
use crate::util::phys::PhysLayer;
use crate::util::session::{RunError, Session, StopReason};

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
        let _ = self.run_inner().await;
        self.listener.update(PortState::Shutdown).get().await;
    }

    async fn run_inner(&mut self) -> Result<(), Shutdown> {
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
                    let delay = self.back_off.on_failure();
                    tracing::warn!("{} - waiting {} ms to re-open port", err, delay.as_millis());
                    self.listener.update(PortState::Wait(delay)).get().await;
                    self.session.wait_for_retry(delay).await?;
                }
                Ok(serial) => {
                    self.back_off.on_success();
                    tracing::info!("serial port open");
                    let mut io = PhysLayer::Serial(serial);
                    self.listener.update(PortState::Open).get().await;
                    match self.session.run(&mut io).await {
                        RunError::Stop(x) => {
                            return Err(x);
                        }
                        RunError::Link(err) => {
                            let delay = self.back_off.on_failure();
                            // we wait here to prevent any kind of rapid retry scenario if the port opens and then immediately fails
                            tracing::warn!(
                                "{} - waiting {} ms to re-open port",
                                err,
                                delay.as_millis()
                            );
                            self.listener.update(PortState::Wait(delay)).get().await;
                            self.session.wait_for_retry(delay).await?;
                        }
                    }
                }
            }
        }
    }
}
