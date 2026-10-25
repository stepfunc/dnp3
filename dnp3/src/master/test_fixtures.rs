// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Handlers and associations shared by the master's tests

use crate::app::Timestamp;
use crate::link::EndpointAddress;
use crate::master::association::{Association, AssociationConfig};
use crate::master::{AssociationHandler, AssociationInformation, ReadHandler};
use crate::phys::PhysAddr;
use crate::transport::FragmentAddr;

/// A handler that ignores every callback and has no system time
pub(crate) struct NullHandler;
impl ReadHandler for NullHandler {}
impl AssociationHandler for NullHandler {}
impl AssociationInformation for NullHandler {}

pub(crate) fn null() -> Box<NullHandler> {
    Box::new(NullHandler)
}

/// The system time [`FixedTime`] reports
pub(crate) const SYSTEM_TIME: Timestamp = Timestamp::new(1_700_000_000_000);

/// An association handler that always reports [`SYSTEM_TIME`]
pub(crate) struct FixedTime;

impl AssociationHandler for FixedTime {
    fn get_current_time(&self) -> Option<Timestamp> {
        Some(SYSTEM_TIME)
    }
}

/// The address of the outstation at `link`, with no physical address
pub(crate) fn fragment_addr(link: EndpointAddress) -> FragmentAddr {
    FragmentAddr {
        link,
        phys: PhysAddr::None,
    }
}

/// An association with the outstation at `link`, whose read handler and association information
/// ignore every callback
pub(crate) fn association(
    link: EndpointAddress,
    config: AssociationConfig,
    handler: Box<dyn AssociationHandler>,
) -> Association {
    Association::new(fragment_addr(link), config, null(), handler, null())
}
