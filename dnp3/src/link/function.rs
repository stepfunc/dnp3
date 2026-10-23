// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) enum Function {
    PriResetLinkStates,
    PriTestLinkStates,
    PriConfirmedUserData,
    PriUnconfirmedUserData,
    PriRequestLinkStatus,
    SecAck,
    SecNack,
    SecLinkStatus,
    SecNotSupported,
    Unknown(u8),
}

mod constants {
    pub(crate) const PRI_RESET_LINK_STATES: u8 = 0x40;
    pub(crate) const PRI_TEST_LINK_STATES: u8 = 0x42;
    pub(crate) const PRI_CONFIRMED_USER_DATA: u8 = 0x43;
    pub(crate) const PRI_UNCONFIRMED_USER_DATA: u8 = 0x44;
    pub(crate) const PRI_REQUEST_LINK_STATUS: u8 = 0x49;
    pub(crate) const SEC_ACK: u8 = 0x00;
    pub(crate) const SEC_NACK: u8 = 0x01;
    pub(crate) const SEC_LINK_STATUS: u8 = 0x0B;
    pub(crate) const SEC_NOT_SUPPORTED: u8 = 0x0F;
}

impl Function {
    pub(crate) fn from(byte: u8) -> Function {
        match byte {
            constants::PRI_RESET_LINK_STATES => Function::PriResetLinkStates,
            constants::PRI_TEST_LINK_STATES => Function::PriTestLinkStates,
            constants::PRI_CONFIRMED_USER_DATA => Function::PriConfirmedUserData,
            constants::PRI_UNCONFIRMED_USER_DATA => Function::PriUnconfirmedUserData,
            constants::PRI_REQUEST_LINK_STATUS => Function::PriRequestLinkStatus,
            constants::SEC_ACK => Function::SecAck,
            constants::SEC_NACK => Function::SecNack,
            constants::SEC_LINK_STATUS => Function::SecLinkStatus,
            constants::SEC_NOT_SUPPORTED => Function::SecNotSupported,
            _ => Function::Unknown(byte),
        }
    }

    pub(crate) fn to_u8(self) -> u8 {
        match self {
            Function::PriResetLinkStates => constants::PRI_RESET_LINK_STATES,
            Function::PriTestLinkStates => constants::PRI_TEST_LINK_STATES,
            Function::PriConfirmedUserData => constants::PRI_CONFIRMED_USER_DATA,
            Function::PriUnconfirmedUserData => constants::PRI_UNCONFIRMED_USER_DATA,
            Function::PriRequestLinkStatus => constants::PRI_REQUEST_LINK_STATUS,
            Function::SecAck => constants::SEC_ACK,
            Function::SecNack => constants::SEC_NACK,
            Function::SecLinkStatus => constants::SEC_LINK_STATUS,
            Function::SecNotSupported => constants::SEC_NOT_SUPPORTED,
            Function::Unknown(x) => x,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const FUNCTIONS: &[(Function, u8)] = &[
        (Function::PriResetLinkStates, 0x40),
        (Function::PriTestLinkStates, 0x42),
        (Function::PriConfirmedUserData, 0x43),
        (Function::PriUnconfirmedUserData, 0x44),
        (Function::PriRequestLinkStatus, 0x49),
        (Function::SecAck, 0x00),
        (Function::SecNack, 0x01),
        (Function::SecLinkStatus, 0x0B),
        (Function::SecNotSupported, 0x0F),
    ];

    #[test]
    fn round_trips_every_byte() {
        for byte in 0..=u8::MAX {
            assert_eq!(Function::from(byte).to_u8(), byte, "{byte:#04X}");
        }
    }

    #[test]
    fn values_match_the_table() {
        for &(function, value) in FUNCTIONS {
            assert_eq!(Function::from(value), function, "{value:#04X}");
            assert_eq!(function.to_u8(), value, "{function:?}");
        }

        // every value missing from the table is captured as unknown
        for byte in 0..=u8::MAX {
            if !FUNCTIONS.iter().any(|&(_, value)| value == byte) {
                assert_eq!(Function::from(byte), Function::Unknown(byte));
            }
        }
    }
}
