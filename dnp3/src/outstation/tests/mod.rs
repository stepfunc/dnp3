use crate::entry::EndpointAddress;
use crate::outstation::task::OutstationConfig;

pub(crate) mod harness;

pub(crate) fn get_default_config() -> OutstationConfig {
    OutstationConfig::new(
        EndpointAddress::from(10).unwrap(),
        EndpointAddress::from(1).unwrap(),
    )
}

/// control functionality
mod controls;
/// clear restart IIN + cold/warm restart
mod restart;
/// time synchronization
mod time;
