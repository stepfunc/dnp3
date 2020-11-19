use super::function::Function;
use crate::entry::LinkAddress;

pub(crate) mod constants {
    pub(crate) const MASK_DIR: u8 = 0x80;
    pub(crate) const MASK_PRM: u8 = 0x40;
    pub(crate) const MASK_FCB: u8 = 0x20;
    pub(crate) const MASK_FCV: u8 = 0x10;
    pub(crate) const MASK_FUNC: u8 = 0x0F;
    pub(crate) const MASK_FUNC_OR_PRM: u8 = MASK_PRM | MASK_FUNC;
    pub(crate) const BROADCAST_CONFIRM_OPTIONAL: u16 = 0xFFFF;
    pub(crate) const BROADCAST_CONFIRM_MANDATORY: u16 = 0xFFFE;
    pub(crate) const BROADCAST_CONFIRM_NOT_REQUIRED: u16 = 0xFFFD;
    pub(crate) const SELF_ADDRESS: u16 = 0xFFFC;
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum BroadcastAddress {
    ConfirmOptional,
    ConfirmMandatory,
    ConfirmNotRequired,
}

impl BroadcastAddress {
    pub(crate) fn value(&self) -> u16 {
        match self {
            BroadcastAddress::ConfirmOptional => constants::BROADCAST_CONFIRM_OPTIONAL,
            BroadcastAddress::ConfirmMandatory => constants::BROADCAST_CONFIRM_MANDATORY,
            BroadcastAddress::ConfirmNotRequired => constants::BROADCAST_CONFIRM_NOT_REQUIRED,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum AnyAddress {
    Normal(LinkAddress),
    Broadcast(BroadcastAddress),
    SelfAddress,
}

impl AnyAddress {
    pub(crate) const fn from(address: u16) -> Self {
        match address {
            constants::BROADCAST_CONFIRM_OPTIONAL => {
                Self::Broadcast(BroadcastAddress::ConfirmOptional)
            }
            constants::BROADCAST_CONFIRM_MANDATORY => {
                Self::Broadcast(BroadcastAddress::ConfirmMandatory)
            }
            constants::BROADCAST_CONFIRM_NOT_REQUIRED => {
                Self::Broadcast(BroadcastAddress::ConfirmNotRequired)
            }
            constants::SELF_ADDRESS => Self::SelfAddress,
            _ => Self::Normal(LinkAddress::raw(address)),
        }
    }

    pub(crate) fn get_normal_address(&self) -> Option<LinkAddress> {
        match self {
            AnyAddress::Normal(x) => Some(*x),
            _ => None,
        }
    }

    pub(crate) fn value(&self) -> u16 {
        match self {
            Self::Normal(x) => x.value(),
            Self::Broadcast(x) => x.value(),
            Self::SelfAddress => constants::SELF_ADDRESS,
        }
    }
}

impl std::fmt::Display for AnyAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AnyAddress::Normal(x) => write!(f, "normal address ({})", x.value()),
            AnyAddress::SelfAddress => write!(f, "self address ({})", constants::SELF_ADDRESS),
            AnyAddress::Broadcast(details) => write!(f, "broadcast address ({})", details.value()),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) struct ControlField {
    pub(crate) func: Function,
    pub(crate) master: bool,
    pub(crate) fcb: bool,
    pub(crate) fcv: bool,
}

impl ControlField {
    pub(crate) fn new(master: bool, function: Function) -> Self {
        Self {
            func: function,
            master,
            fcb: false,
            fcv: false,
        }
    }

    pub(crate) fn from(byte: u8) -> ControlField {
        ControlField {
            func: Function::from(byte & constants::MASK_FUNC_OR_PRM),
            master: (byte & constants::MASK_DIR) != 0,
            fcb: (byte & constants::MASK_FCB) != 0,
            fcv: (byte & constants::MASK_FCV) != 0,
        }
    }

    pub(crate) fn to_u8(self) -> u8 {
        let mut ret = 0;
        ret |= if self.master { constants::MASK_DIR } else { 0 };
        // the PRM bit is part of the function code
        ret |= if self.fcb { constants::MASK_FCB } else { 0 };
        ret |= if self.fcv { constants::MASK_FCV } else { 0 };
        ret |= self.func.to_u8();
        ret
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) struct AddressPair {
    pub(crate) destination: AnyAddress,
    pub(crate) source: AnyAddress,
}

impl AddressPair {
    pub(crate) fn new(destination: AnyAddress, source: AnyAddress) -> Self {
        Self {
            destination,
            source,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) struct Header {
    pub(crate) control: ControlField,
    pub(crate) addresses: AddressPair,
}

impl Header {
    pub(crate) fn new(control: ControlField, addresses: AddressPair) -> Header {
        Header { control, addresses }
    }
}
