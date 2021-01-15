use crate::link::header::AnyAddress;

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct EndpointAddress {
    address: u16,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SpecialAddressError {
    pub address: u16,
}

impl std::error::Error for SpecialAddressError {}

impl std::fmt::Display for SpecialAddressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "special address ({}) may not be used as a master or outstation address",
            self.address
        )
    }
}

impl EndpointAddress {
    pub fn from(value: u16) -> Result<Self, SpecialAddressError> {
        match AnyAddress::from(value) {
            AnyAddress::Endpoint(x) => Ok(x),
            _ => Err(SpecialAddressError { address: value }),
        }
    }

    pub(crate) const fn raw(address: u16) -> EndpointAddress {
        EndpointAddress { address }
    }

    pub fn raw_value(&self) -> u16 {
        self.address
    }

    pub(crate) fn wrap(&self) -> AnyAddress {
        AnyAddress::Endpoint(*self)
    }
}

impl std::fmt::Display for EndpointAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.address)
    }
}

/// entry points for creating and spawning master tasks
pub mod master;
/// entry points for creating and spawning outstation tasks
pub mod outstation;
