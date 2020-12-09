use crate::entry::EndpointAddress;
use crate::outstation::config::OutstationConfig;

pub(crate) mod harness;

pub(crate) fn get_default_config() -> OutstationConfig {
    OutstationConfig::new(
        EndpointAddress::from(10).unwrap(),
        EndpointAddress::from(1).unwrap(),
    )
}

/// control functionality
mod controls;
/// state machine for READ requests
mod read_states;
/// clear restart IIN + cold/warm restart
mod restart;
/// time synchronization
mod time;
