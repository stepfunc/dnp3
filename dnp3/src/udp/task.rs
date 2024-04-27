use crate::app::{Shutdown, Timeout};
use crate::udp::layer::UdpFactory;
use crate::util::session::{RunError, Session, StopReason};

pub(crate) struct UdpTask {
    pub(crate) session: Session,
    pub(crate) factory: UdpFactory,
    pub(crate) retry_delay: Timeout,
}

enum Delay {
    Yes,
    No,
}

impl UdpTask {
    pub(crate) async fn run(mut self) -> Result<(), Shutdown> {
        loop {
            self.session.wait_for_enabled().await?;
            if let Delay::Yes = self.run_one().await? {
                if let Err(reason) = self.session.wait_for_retry(self.retry_delay.into()).await {
                    Self::handle_stop(reason)?;
                }
            }
        }
    }

    fn handle_stop(reason: StopReason) -> Result<Delay, Shutdown> {
        match reason {
            StopReason::Disable => Ok(Delay::No),
            StopReason::Shutdown => Err(Shutdown),
        }
    }

    async fn run_one(&mut self) -> Result<Delay, Shutdown> {
        match self.factory.open().await {
            Ok(mut phys) => match self.session.run(&mut phys).await {
                RunError::Stop(r) => Self::handle_stop(r),
                RunError::Link(err) => {
                    tracing::warn!("{err}");
                    Ok(Delay::Yes)
                }
            },
            Err(err) => {
                tracing::error!("{err}");
                Ok(Delay::Yes)
            }
        }
    }
}
