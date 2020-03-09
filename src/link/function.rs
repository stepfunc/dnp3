
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Function
{
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
    pub const PRI_RESET_LINK_STATES : u8 = 0x40;
    pub const PRI_TEST_LINK_STATES : u8 = 0x42;
    pub const PRI_CONFIRMED_USER_DATA : u8  = 0x43;
    pub const PRI_UNCONFIRMED_USER_DATA : u8  = 0x44;
    pub const PRI_REQUEST_LINK_STATUS : u8  = 0x49;
    pub const SEC_ACK : u8  = 0x00;
    pub const SEC_NACK : u8  = 0x01;
    pub const SEC_LINK_STATUS : u8  = 0x0B;
    pub const SEC_NOT_SUPPORTED : u8  = 0x0F;
}

impl Function {

    pub fn from(byte: u8) -> Function {
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

}