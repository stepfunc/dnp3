// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::Timeout;
use crate::link::reader::LinkModes;
use crate::link::{LinkErrorMode, LinkReadMode};
use crate::outstation::session::task::OutstationTask;
use crate::outstation::{
    ControlHandler, OutstationApplication, OutstationConfig, OutstationHandle,
    OutstationInformation,
};
use crate::udp::layer::UdpFactory;
use crate::udp::task::UdpTask;
use crate::udp::UdpSocketMode;
use crate::util::lifecycle::Enabled;
use crate::util::phys::PhysAddr;
use crate::util::session::Session;
use std::net::SocketAddr;
use tracing::Instrument;

/// UDP outstation configuration
#[derive(Copy, Clone, Debug)]
pub struct OutstationUdpConfig {
    /// Local endpoint to which the UDP socket is bound
    pub local_endpoint: SocketAddr,
    /// Remote endpoint
    pub remote_endpoint: SocketAddr,
    /// UDP socket mode to use
    pub socket_mode: UdpSocketMode,
    /// Read mode to use, this should typically be set to [`LinkReadMode::Datagram`].
    pub link_read_mode: LinkReadMode,
    /// Period to wait before retrying after a failure to bind/connect the UDP socket
    pub retry_delay: Timeout,
}

impl OutstationUdpConfig {
    fn factory(&self) -> UdpFactory {
        match self.socket_mode {
            UdpSocketMode::OneToOne => {
                UdpFactory::connected(self.local_endpoint, self.remote_endpoint)
            }
            UdpSocketMode::OneToMany => UdpFactory::bound(self.local_endpoint),
        }
    }

    fn link_modes(&self) -> LinkModes {
        match self.link_read_mode {
            LinkReadMode::Stream => LinkModes::stream(LinkErrorMode::Discard),
            LinkReadMode::Datagram => LinkModes::datagram(LinkErrorMode::Discard),
        }
    }
}

/// Spawn an outstation task onto the `Tokio` runtime that reads and writes from a UDP socket. The task runs until the returned handle is dropped.
///
/// With [`UdpSocketMode::OneToOne`], the outstation will **only** read/write
/// datagrams to/from the `remote_endpoint`. With [`UdpSocketMode::OneToMany`],
/// it accepts datagrams from any source and responds to the sender, using the `remote_endpoint` only for unsolicited responses.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// Use Runtime::enter() if required.
pub fn spawn_outstation_udp(
    udp_config: OutstationUdpConfig,
    config: OutstationConfig,
    application: Box<dyn OutstationApplication>,
    information: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
) -> OutstationHandle {
    let (task, handle) = OutstationTask::create(
        Enabled::Yes,
        udp_config.link_modes(),
        config,
        PhysAddr::Udp(udp_config.remote_endpoint),
        application,
        information,
        control_handler,
    );

    let task = UdpTask {
        session: Session::outstation(task),
        factory: udp_config.factory(),
        retry_delay: udp_config.retry_delay,
    };

    let future = async move {
        task.run()
            .instrument(
                tracing::info_span!("dnp3-outstation-udp", "endpoint" = ?udp_config.local_endpoint),
            )
            .await;
    };
    tokio::spawn(future);
    handle
}
