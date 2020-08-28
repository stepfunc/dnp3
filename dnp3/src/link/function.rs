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
