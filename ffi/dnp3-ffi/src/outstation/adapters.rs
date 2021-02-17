use dnp3::app::control::*;
use dnp3::app::*;
use dnp3::outstation::database::Database;
use dnp3::outstation::*;

use crate::ffi;

impl OutstationApplication for ffi::OutstationApplication {
    fn get_processing_delay_ms(&self) -> u16 {
        ffi::OutstationApplication::get_processing_delay_ms(self).unwrap_or(0)
    }

    fn cold_restart(&mut self) -> Option<RestartDelay> {
        ffi::OutstationApplication::cold_restart(self)
            .map(|delay| delay.into())
            .flatten()
    }

    fn warm_restart(&mut self) -> Option<RestartDelay> {
        ffi::OutstationApplication::warm_restart(self)
            .map(|delay| delay.into())
            .flatten()
    }
}

impl From<ffi::RestartDelay> for Option<RestartDelay> {
    fn from(from: ffi::RestartDelay) -> Self {
        match from.restart_type() {
            ffi::RestartDelayType::NotSupported => None,
            ffi::RestartDelayType::Seconds => Some(RestartDelay::Seconds(from.value())),
            ffi::RestartDelayType::Milliseconds => Some(RestartDelay::Milliseconds(from.value())),
        }
    }
}

impl OutstationInformation for ffi::OutstationInformation {
    fn process_request_from_idle(&mut self, header: RequestHeader) {
        ffi::OutstationInformation::process_request_from_idle(self, header.into());
    }

    fn broadcast_received(&mut self, function: FunctionCode, action: BroadcastAction) {
        ffi::OutstationInformation::broadcast_received(self, function.into(), action.into());
    }

    fn enter_solicited_confirm_wait(&mut self, ecsn: Sequence) {
        ffi::OutstationInformation::enter_solicited_confirm_wait(self, ecsn.value());
    }

    fn solicited_confirm_timeout(&mut self, ecsn: Sequence) {
        ffi::OutstationInformation::solicited_confirm_timeout(self, ecsn.value());
    }

    fn solicited_confirm_received(&mut self, ecsn: Sequence) {
        ffi::OutstationInformation::solicited_confirm_received(self, ecsn.value());
    }

    fn solicited_confirm_wait_new_request(&mut self) {
        ffi::OutstationInformation::solicited_confirm_wait_new_request(self);
    }

    fn wrong_solicited_confirm_seq(&mut self, ecsn: Sequence, seq: Sequence) {
        ffi::OutstationInformation::wrong_solicited_confirm_seq(self, ecsn.value(), seq.value());
    }

    fn unexpected_confirm(&mut self, unsolicited: bool, seq: Sequence) {
        ffi::OutstationInformation::unexpected_confirm(self, unsolicited, seq.value());
    }

    fn enter_unsolicited_confirm_wait(&mut self, ecsn: Sequence) {
        ffi::OutstationInformation::enter_unsolicited_confirm_wait(self, ecsn.value());
    }

    fn unsolicited_confirm_timeout(&mut self, ecsn: Sequence, retry: bool) {
        ffi::OutstationInformation::unsolicited_confirm_timeout(self, ecsn.value(), retry);
    }

    fn unsolicited_confirmed(&mut self, ecsn: Sequence) {
        ffi::OutstationInformation::unsolicited_confirmed(self, ecsn.value());
    }

    fn clear_restart_iin(&mut self) {
        ffi::OutstationInformation::clear_restart_iin(self);
    }
}

impl ControlHandler for ffi::ControlHandler {
    fn begin_fragment(&mut self) {
        ffi::ControlHandler::begin_fragment(self);
    }

    fn end_fragment(&mut self) {
        ffi::ControlHandler::end_fragment(self);
    }
}

impl ControlSupport<Group12Var1> for ffi::ControlHandler {
    fn select(
        &mut self,
        control: Group12Var1,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        // TODO: pass database
        ffi::ControlHandler::select_g12v1(self, control.into(), index, database as *mut _)
            .map(|e| e.into())
            .unwrap_or(CommandStatus::NotSupported)
    }

    fn operate(
        &mut self,
        control: Group12Var1,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        // TODO: pass database
        ffi::ControlHandler::operate_g12v1(
            self,
            control.into(),
            index,
            op_type.into(),
            database as *mut _,
        )
        .map(|e| e.into())
        .unwrap_or(CommandStatus::NotSupported)
    }
}

impl ControlSupport<Group41Var1> for ffi::ControlHandler {
    fn select(
        &mut self,
        control: Group41Var1,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        // TODO: pass database
        ffi::ControlHandler::select_g41v1(self, control.value, index, database as *mut _)
            .map(|e| e.into())
            .unwrap_or(CommandStatus::NotSupported)
    }

    fn operate(
        &mut self,
        control: Group41Var1,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        // TODO: pass database
        ffi::ControlHandler::operate_g41v1(
            self,
            control.value,
            index,
            op_type.into(),
            database as *mut _,
        )
        .map(|e| e.into())
        .unwrap_or(CommandStatus::NotSupported)
    }
}

impl ControlSupport<Group41Var2> for ffi::ControlHandler {
    fn select(
        &mut self,
        control: Group41Var2,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        // TODO: pass database
        ffi::ControlHandler::select_g41v2(self, control.value, index, database as *mut _)
            .map(|e| e.into())
            .unwrap_or(CommandStatus::NotSupported)
    }

    fn operate(
        &mut self,
        control: Group41Var2,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        // TODO: pass database
        ffi::ControlHandler::operate_g41v2(
            self,
            control.value,
            index,
            op_type.into(),
            database as *mut _,
        )
        .map(|e| e.into())
        .unwrap_or(CommandStatus::NotSupported)
    }
}

impl ControlSupport<Group41Var3> for ffi::ControlHandler {
    fn select(
        &mut self,
        control: Group41Var3,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        // TODO: pass database
        ffi::ControlHandler::select_g41v3(self, control.value, index, database as *mut _)
            .map(|e| e.into())
            .unwrap_or(CommandStatus::NotSupported)
    }

    fn operate(
        &mut self,
        control: Group41Var3,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        // TODO: pass database
        ffi::ControlHandler::operate_g41v3(
            self,
            control.value,
            index,
            op_type.into(),
            database as *mut _,
        )
        .map(|e| e.into())
        .unwrap_or(CommandStatus::NotSupported)
    }
}

impl ControlSupport<Group41Var4> for ffi::ControlHandler {
    fn select(
        &mut self,
        control: Group41Var4,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        // TODO: pass database
        ffi::ControlHandler::select_g41v4(self, control.value, index, database as *mut _)
            .map(|e| e.into())
            .unwrap_or(CommandStatus::NotSupported)
    }

    fn operate(
        &mut self,
        control: Group41Var4,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        // TODO: pass database
        ffi::ControlHandler::operate_g41v4(
            self,
            control.value,
            index,
            op_type.into(),
            database as *mut _,
        )
        .map(|e| e.into())
        .unwrap_or(CommandStatus::NotSupported)
    }
}

impl From<RequestHeader> for ffi::RequestHeader {
    fn from(from: RequestHeader) -> ffi::RequestHeader {
        ffi::RequestHeaderFields {
            control: from.control.into(),
            function: from.function.into(),
        }
        .into()
    }
}

impl From<ControlField> for ffi::Control {
    fn from(from: ControlField) -> Self {
        ffi::ControlFields {
            fir: from.fir,
            fin: from.fin,
            con: from.con,
            uns: from.uns,
            seq: from.seq.value(),
        }
        .into()
    }
}

impl From<BroadcastAction> for ffi::BroadcastAction {
    fn from(from: BroadcastAction) -> Self {
        match from {
            BroadcastAction::Processed => Self::Processed,
            BroadcastAction::IgnoredByConfiguration => Self::IgnoredByConfiguration,
            BroadcastAction::BadObjectHeaders => Self::BadObjectHeaders,
            BroadcastAction::UnsupportedFunction(_) => Self::UnsupportedFunction,
        }
    }
}

impl From<FunctionCode> for ffi::FunctionCode {
    fn from(from: FunctionCode) -> Self {
        match from {
            FunctionCode::Confirm => ffi::FunctionCode::Confirm,
            FunctionCode::Read => ffi::FunctionCode::Read,
            FunctionCode::Write => ffi::FunctionCode::Write,
            FunctionCode::Select => ffi::FunctionCode::Select,
            FunctionCode::Operate => ffi::FunctionCode::Operate,
            FunctionCode::DirectOperate => ffi::FunctionCode::DirectOperate,
            FunctionCode::DirectOperateNoResponse => ffi::FunctionCode::DirectOperateNoResponse,
            FunctionCode::ImmediateFreeze => ffi::FunctionCode::ImmediateFreeze,
            FunctionCode::ImmediateFreezeNoResponse => ffi::FunctionCode::ImmediateFreezeNoResponse,
            FunctionCode::FreezeClear => ffi::FunctionCode::FreezeClear,
            FunctionCode::FreezeClearNoResponse => ffi::FunctionCode::FreezeClearNoResponse,
            FunctionCode::FreezeAtTime => ffi::FunctionCode::FreezeAtTime,
            FunctionCode::FreezeAtTimeNoResponse => ffi::FunctionCode::FreezeAtTimeNoResponse,
            FunctionCode::ColdRestart => ffi::FunctionCode::ColdRestart,
            FunctionCode::WarmRestart => ffi::FunctionCode::WarmRestart,
            FunctionCode::InitializeData => ffi::FunctionCode::InitializeData,
            FunctionCode::InitializeApplication => ffi::FunctionCode::InitializeApplication,
            FunctionCode::StartApplication => ffi::FunctionCode::StartApplication,
            FunctionCode::StopApplication => ffi::FunctionCode::StopApplication,
            FunctionCode::SaveConfiguration => ffi::FunctionCode::SaveConfiguration,
            FunctionCode::EnableUnsolicited => ffi::FunctionCode::EnableUnsolicited,
            FunctionCode::DisableUnsolicited => ffi::FunctionCode::DisableUnsolicited,
            FunctionCode::AssignClass => ffi::FunctionCode::AssignClass,
            FunctionCode::DelayMeasure => ffi::FunctionCode::DelayMeasure,
            FunctionCode::RecordCurrentTime => ffi::FunctionCode::RecordCurrentTime,
            FunctionCode::OpenFile => ffi::FunctionCode::OpenFile,
            FunctionCode::CloseFile => ffi::FunctionCode::CloseFile,
            FunctionCode::DeleteFile => ffi::FunctionCode::DeleteFile,
            FunctionCode::GetFileInfo => ffi::FunctionCode::GetFileInfo,
            FunctionCode::AuthenticateFile => ffi::FunctionCode::AuthenticateFile,
            FunctionCode::AbortFile => ffi::FunctionCode::AbortFile,
            FunctionCode::Response => ffi::FunctionCode::Response,
            FunctionCode::UnsolicitedResponse => ffi::FunctionCode::UnsolicitedResponse,
        }
    }
}

impl From<Group12Var1> for ffi::G12v1 {
    fn from(from: Group12Var1) -> Self {
        ffi::G12v1Fields {
            code: from.code.into(),
            count: from.count,
            on_time: from.on_time,
            off_time: from.off_time,
        }
        .into()
    }
}

impl From<ControlCode> for ffi::ControlCode {
    fn from(from: ControlCode) -> Self {
        ffi::ControlCodeFields {
            tcc: from.tcc.into(),
            clear: from.clear,
            queue: from.queue,
            op_type: from.op_type.into(),
        }
        .into()
    }
}

impl From<TripCloseCode> for ffi::TripCloseCode {
    fn from(from: TripCloseCode) -> Self {
        match from {
            TripCloseCode::Nul => Self::Nul,
            TripCloseCode::Close => Self::Close,
            TripCloseCode::Trip => Self::Trip,
            TripCloseCode::Reserved => Self::Reserved,
            TripCloseCode::Unknown(_) => Self::Nul, // TODO: do something better than this
        }
    }
}

impl From<OpType> for ffi::OpType {
    fn from(from: OpType) -> Self {
        match from {
            OpType::Nul => Self::Nul,
            OpType::PulseOn => Self::PulseOn,
            OpType::PulseOff => Self::PulseOff,
            OpType::LatchOn => Self::LatchOn,
            OpType::LatchOff => Self::LatchOff,
            OpType::Unknown(_) => Self::Nul, // TODO: do something better than this
        }
    }
}

impl From<OperateType> for ffi::OperateType {
    fn from(from: OperateType) -> Self {
        match from {
            OperateType::SelectBeforeOperate => Self::SelectBeforeOperate,
            OperateType::DirectOperate => Self::DirectOperate,
            OperateType::DirectOperateNoAck => Self::DirectOperateNoAck,
        }
    }
}

impl From<CommandStatus> for ffi::CommandStatus {
    fn from(from: CommandStatus) -> Self {
        match from {
            CommandStatus::Success => Self::Success,
            CommandStatus::Timeout => Self::Timeout,
            CommandStatus::NoSelect => Self::NoSelect,
            CommandStatus::FormatError => Self::FormatError,
            CommandStatus::NotSupported => Self::NotSupported,
            CommandStatus::AlreadyActive => Self::AlreadyActive,
            CommandStatus::HardwareError => Self::HardwareError,
            CommandStatus::Local => Self::Local,
            CommandStatus::TooManyOps => Self::TooManyOps,
            CommandStatus::NotAuthorized => Self::NotAuthorized,
            CommandStatus::AutomationInhibit => Self::AutomationInhibit,
            CommandStatus::ProcessingLimited => Self::ProcessingLimited,
            CommandStatus::OutOfRange => Self::OutOfRange,
            CommandStatus::DownstreamLocal => Self::DownstreamLocal,
            CommandStatus::AlreadyComplete => Self::AlreadyComplete,
            CommandStatus::Blocked => Self::Blocked,
            CommandStatus::Canceled => Self::Canceled,
            CommandStatus::BlockedOtherMaster => Self::BlockedOtherMaster,
            CommandStatus::DownstreamFail => Self::DownstreamFail,
            CommandStatus::NonParticipating => Self::NonParticipating,
            CommandStatus::Unknown(_) => Self::Unknown,
        }
    }
}

impl From<ffi::CommandStatus> for CommandStatus {
    fn from(from: ffi::CommandStatus) -> Self {
        match from {
            ffi::CommandStatus::Success => Self::Success,
            ffi::CommandStatus::Timeout => Self::Timeout,
            ffi::CommandStatus::NoSelect => Self::NoSelect,
            ffi::CommandStatus::FormatError => Self::FormatError,
            ffi::CommandStatus::NotSupported => Self::NotSupported,
            ffi::CommandStatus::AlreadyActive => Self::AlreadyActive,
            ffi::CommandStatus::HardwareError => Self::HardwareError,
            ffi::CommandStatus::Local => Self::Local,
            ffi::CommandStatus::TooManyOps => Self::TooManyOps,
            ffi::CommandStatus::NotAuthorized => Self::NotAuthorized,
            ffi::CommandStatus::AutomationInhibit => Self::AutomationInhibit,
            ffi::CommandStatus::ProcessingLimited => Self::ProcessingLimited,
            ffi::CommandStatus::OutOfRange => Self::OutOfRange,
            ffi::CommandStatus::DownstreamLocal => Self::DownstreamLocal,
            ffi::CommandStatus::AlreadyComplete => Self::AlreadyComplete,
            ffi::CommandStatus::Blocked => Self::Blocked,
            ffi::CommandStatus::Canceled => Self::Canceled,
            ffi::CommandStatus::BlockedOtherMaster => Self::BlockedOtherMaster,
            ffi::CommandStatus::DownstreamFail => Self::DownstreamFail,
            ffi::CommandStatus::NonParticipating => Self::NonParticipating,
            ffi::CommandStatus::Unknown => Self::Unknown(0), // TODO: do something better than this
        }
    }
}
