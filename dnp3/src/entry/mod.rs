use crate::link::header::AnyAddress;

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct LinkAddress {
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

impl LinkAddress {
    pub fn from(value: u16) -> Result<Self, SpecialAddressError> {
        match AnyAddress::from(value) {
            AnyAddress::Normal(x) => Ok(x),
            _ => Err(SpecialAddressError { address: value }),
        }
    }

    pub(crate) const fn raw(address: u16) -> LinkAddress {
        LinkAddress { address }
    }

    pub(crate) fn value(&self) -> u16 {
        self.address
    }

    pub(crate) fn wrap(&self) -> AnyAddress {
        AnyAddress::Normal(*self)
    }
}

impl std::fmt::Display for LinkAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.address)
    }
}

/// entry points for creating and spawning master tasks
pub mod master {
    /// entry points for creating and spawning TCP-based master tasks
    pub mod tcp;
}
