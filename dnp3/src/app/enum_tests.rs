// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Wire values of the generated enums in `app_enums.rs` and `control_enums.rs`, and the table
//! check that the enums with a catch-all variant share

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

/// Check an enum's wire values against its table in both directions, that every value missing
/// from the table converts to the catch-all variant, and that every value converts back to itself
pub(crate) fn check_wire_table<E, V>(
    table: &[(E, V)],
    every_value: impl IntoIterator<Item = V>,
    from_value: impl Fn(V) -> E,
    to_value: impl Fn(E) -> V,
    catch_all: impl Fn(V) -> E,
) where
    E: Copy + PartialEq + std::fmt::Debug,
    V: Copy + PartialEq + std::fmt::Debug,
{
    for &(variant, value) in table {
        assert_eq!(from_value(value), variant, "{value:?}");
        assert_eq!(to_value(variant), value, "{variant:?}");
    }

    for value in every_value {
        assert_eq!(to_value(from_value(value)), value);
        if !table.iter().any(|&(_, x)| x == value) {
            assert_eq!(from_value(value), catch_all(value));
        }
    }
}

#[test]
fn command_status_values_match_the_table() {
    check_wire_table(
        COMMAND_STATUS,
        0..=u8::MAX,
        CommandStatus::from,
        CommandStatus::as_u8,
        CommandStatus::Unknown,
    );
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
