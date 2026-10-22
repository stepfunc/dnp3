//! Wire values of the generated enums in `app_enums.rs` and `control_enums.rs`

use crate::app::control::CommandStatus;
use crate::app::FunctionCode;

const COMMAND_STATUS: &[(CommandStatus, u8)] = &[
    (CommandStatus::Success, 0),
    (CommandStatus::Timeout, 1),
    (CommandStatus::NoSelect, 2),
    (CommandStatus::FormatError, 3),
    (CommandStatus::NotSupported, 4),
    (CommandStatus::AlreadyActive, 5),
    (CommandStatus::HardwareError, 6),
    (CommandStatus::Local, 7),
    (CommandStatus::TooManyOps, 8),
    (CommandStatus::NotAuthorized, 9),
    (CommandStatus::AutomationInhibit, 10),
    (CommandStatus::ProcessingLimited, 11),
    (CommandStatus::OutOfRange, 12),
    (CommandStatus::DownstreamLocal, 13),
    (CommandStatus::AlreadyComplete, 14),
    (CommandStatus::Blocked, 15),
    (CommandStatus::Canceled, 16),
    (CommandStatus::BlockedOtherMaster, 17),
    (CommandStatus::DownstreamFail, 18),
    (CommandStatus::NonParticipating, 126),
];

const FUNCTION_CODES: &[(FunctionCode, u8)] = &[
    (FunctionCode::Confirm, 0),
    (FunctionCode::Read, 1),
    (FunctionCode::Write, 2),
    (FunctionCode::Select, 3),
    (FunctionCode::Operate, 4),
    (FunctionCode::DirectOperate, 5),
    (FunctionCode::DirectOperateNoResponse, 6),
    (FunctionCode::ImmediateFreeze, 7),
    (FunctionCode::ImmediateFreezeNoResponse, 8),
    (FunctionCode::FreezeClear, 9),
    (FunctionCode::FreezeClearNoResponse, 10),
    (FunctionCode::FreezeAtTime, 11),
    (FunctionCode::FreezeAtTimeNoResponse, 12),
    (FunctionCode::ColdRestart, 13),
    (FunctionCode::WarmRestart, 14),
    (FunctionCode::InitializeData, 15),
    (FunctionCode::InitializeApplication, 16),
    (FunctionCode::StartApplication, 17),
    (FunctionCode::StopApplication, 18),
    (FunctionCode::SaveConfiguration, 19),
    (FunctionCode::EnableUnsolicited, 20),
    (FunctionCode::DisableUnsolicited, 21),
    (FunctionCode::AssignClass, 22),
    (FunctionCode::DelayMeasure, 23),
    (FunctionCode::RecordCurrentTime, 24),
    (FunctionCode::OpenFile, 25),
    (FunctionCode::CloseFile, 26),
    (FunctionCode::DeleteFile, 27),
    (FunctionCode::GetFileInfo, 28),
    (FunctionCode::AuthenticateFile, 29),
    (FunctionCode::AbortFile, 30),
    (FunctionCode::Response, 129),
    (FunctionCode::UnsolicitedResponse, 130),
];

#[test]
fn command_status_round_trips_every_value() {
    for byte in 0..=u8::MAX {
        assert_eq!(CommandStatus::from(byte).as_u8(), byte);
    }
}

#[test]
fn command_status_values_match_the_table() {
    for &(status, value) in COMMAND_STATUS {
        assert_eq!(CommandStatus::from(value), status, "{value}");
        assert_eq!(status.as_u8(), value, "{status:?}");
    }

    // every value missing from the table is captured as unknown
    for byte in 0..=u8::MAX {
        if !COMMAND_STATUS.iter().any(|&(_, value)| value == byte) {
            assert_eq!(CommandStatus::from(byte), CommandStatus::Unknown(byte));
        }
    }
}

#[test]
fn function_code_values_match_the_table() {
    for &(function, value) in FUNCTION_CODES {
        assert_eq!(FunctionCode::from(value), Some(function), "{value}");
        assert_eq!(function.as_u8(), value, "{function:?}");
    }

    // every value missing from the table is rejected
    for byte in 0..=u8::MAX {
        if !FUNCTION_CODES.iter().any(|&(_, value)| value == byte) {
            assert_eq!(FunctionCode::from(byte), None, "{byte}");
        }
    }
}
