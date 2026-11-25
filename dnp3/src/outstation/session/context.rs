// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use ::core::time::Duration;

use super::state::{KeepAlive, SessionState};
use crate::app::parse::options::ParseOptions;
use crate::app::{Iin, Iin2, ResponseHeader, Sequence, Timeout};
use crate::link::EndpointAddress;
use crate::outstation::config::{Feature, OutstationConfig};
use crate::outstation::database::DatabaseHandle;
use crate::outstation::traits::{ControlHandler, OutstationApplication, OutstationInformation};
use crate::phys::PhysAddr;
use crate::transport::FragmentAddr;
use crate::util::clock::MonotonicClock;

/// Protocol state, database handle, handlers, and clock owned by [`super::OutstationCore`]
/// separately from its active [`super::phase::Phase`] and response [`super::response::Slots`].
pub(super) struct SessionContext<C: MonotonicClock> {
    pub(super) config: SessionConfig,
    pub(super) parse_options: ParseOptions,
    pub(super) destination: FragmentAddr,
    pub(super) state: SessionState,
    pub(super) keep_alive: KeepAlive,
    pub(super) db: DatabaseHandle,
    pub(super) application: Box<dyn OutstationApplication>,
    pub(super) info: Box<dyn OutstationInformation>,
    pub(super) control_handler: Box<dyn ControlHandler>,
    pub(super) clock: C,
}

impl<C: MonotonicClock> SessionContext<C> {
    pub(super) fn new(
        config: &OutstationConfig,
        phys_addr: PhysAddr,
        db: DatabaseHandle,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
        clock: C,
    ) -> Self {
        let session_config = SessionConfig::from(config);
        let state = SessionState::new(config.read_header_limit(), config.features.unsolicited);
        let destination = FragmentAddr {
            link: config.master_address,
            phys: phys_addr,
        };
        let now = clock.now();
        Self {
            config: session_config,
            parse_options: ParseOptions::get_static(),
            destination,
            state,
            keep_alive: KeepAlive::new(config.keep_alive_timeout, now),
            db,
            application,
            info: information,
            control_handler,
            clock,
        }
    }

    pub(super) fn reset(&mut self) {
        self.db.reset();
        self.state.reset();
    }

    pub(super) fn required_master_address(&self) -> Option<EndpointAddress> {
        match self.config.respond_to_any_master {
            Feature::Enabled => None,
            Feature::Disabled => Some(self.destination.link),
        }
    }

    /// Clear written events on confirmation: `begin_confirm` and `end_confirm` run outside the
    /// database lock, while `event_cleared` runs inside it.
    pub(super) async fn clear_written_events(&mut self) {
        self.application.begin_confirm();
        let state = self
            .db
            .with_database(|db| db.inner.clear_written_events(self.application.as_mut()));
        self.application.end_confirm(state).get().await;
    }
}

pub(super) struct SessionConfig {
    pub(super) confirm_timeout: Timeout,
    pub(super) select_timeout: Timeout,
    pub(super) broadcast: Feature,
    pub(super) respond_to_any_master: Feature,
    pub(super) max_unsolicited_retries: Option<usize>,
    pub(super) unsolicited_retry_delay: Duration,
    pub(super) max_controls_per_request: Option<u16>,
}

impl From<&OutstationConfig> for SessionConfig {
    fn from(config: &OutstationConfig) -> Self {
        SessionConfig {
            confirm_timeout: config.confirm_timeout,
            select_timeout: config.select_timeout,
            broadcast: config.features.broadcast,
            respond_to_any_master: config.features.respond_to_any_master,
            max_unsolicited_retries: config.max_unsolicited_retries,
            unsolicited_retry_delay: config.unsolicited_retry_delay,
            max_controls_per_request: config.max_controls_per_request,
        }
    }
}

/// The header of a solicited response that reports IIN2.2.
pub(super) fn parameter_error(seq: Sequence) -> ResponseHeader {
    ResponseHeader::solicited(seq, Iin::default() | Iin2::PARAMETER_ERROR)
}
