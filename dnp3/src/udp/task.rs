// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::time::Duration;

use crate::app::Timeout;
use crate::link::error::LinkError;
use crate::udp::layer::UdpFactory;
use crate::util::phys::PhysLayer;
use crate::util::session::{run_connection, Connection, Session};

pub(crate) struct UdpTask {
    pub(crate) session: Session,
    pub(crate) factory: UdpFactory,
    pub(crate) retry_delay: Timeout,
}

impl UdpTask {
    pub(crate) async fn run(mut self) {
        run_connection(&mut self).await;
    }
}

impl Connection for UdpTask {
    type Peer = ();

    const CLOSE_AFTER_RETRY_WAIT: bool = false;

    fn session(&mut self) -> &mut Session {
        &mut self.session
    }

    async fn disabled(&mut self) {}

    async fn open(&mut self) -> Result<(PhysLayer, ()), Duration> {
        match self.factory.open().await {
            Ok(phys) => Ok((phys, ())),
            Err(err) => {
                tracing::error!("{err}");
                Err(self.retry_delay.into())
            }
        }
    }

    async fn failed(&mut self, _: (), err: LinkError) -> Duration {
        tracing::warn!("{err}");
        self.retry_delay.into()
    }
}
