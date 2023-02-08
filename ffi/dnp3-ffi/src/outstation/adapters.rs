use crate::attr::FfiAttrValue;
use dnp3::app::attr::{AnyAttribute, Attribute, FloatType};
use dnp3::app::control::*;
use dnp3::app::*;
use dnp3::outstation::database::DatabaseHandle;
use dnp3::outstation::*;
use std::ffi::CString;

use crate::ffi;

impl OutstationApplication for ffi::OutstationApplication {
    fn get_processing_delay_ms(&self) -> u16 {
        ffi::OutstationApplication::get_processing_delay_ms(self).unwrap_or(0)
    }

    fn write_absolute_time(&mut self, time: Timestamp) -> Result<(), RequestError> {
        ffi::OutstationApplication::write_absolute_time(self, time.raw_value())
            .map(|res| res.into())
            .unwrap_or(Err(RequestError::NotSupported))
    }

    fn get_application_iin(&self) -> ApplicationIin {
        ffi::OutstationApplication::get_application_iin(self)
            .map(|iin| iin.into())
            .unwrap_or_default()
    }

    fn cold_restart(&mut self) -> Option<RestartDelay> {
        ffi::OutstationApplication::cold_restart(self).and_then(|delay| delay.into())
    }

    fn warm_restart(&mut self) -> Option<RestartDelay> {
        ffi::OutstationApplication::warm_restart(self).and_then(|delay| delay.into())
    }

    fn freeze_counter(
        &mut self,
        indices: FreezeIndices,
        freeze_type: FreezeType,
        database: &mut DatabaseHandle,
    ) -> Result<(), RequestError> {
        let result: Option<ffi::FreezeResult> = match (indices, freeze_type) {
            (FreezeIndices::All, FreezeType::ImmediateFreeze) => {
                ffi::OutstationApplication::freeze_counters_all(
                    self,
                    ffi::FreezeType::ImmediateFreeze,
                    database as *mut _,
                )
            }
            (FreezeIndices::All, FreezeType::FreezeAndClear) => {
                ffi::OutstationApplication::freeze_counters_all(
                    self,
                    ffi::FreezeType::FreezeAndClear,
                    database as *mut _,
                )
            }
            (FreezeIndices::Range(start, stop), FreezeType::ImmediateFreeze) => {
                ffi::OutstationApplication::freeze_counters_range(
                    self,
                    start,
                    stop,
                    ffi::FreezeType::ImmediateFreeze,
                    database as *mut _,
                )
            }
            (FreezeIndices::Range(start, stop), FreezeType::FreezeAndClear) => {
                ffi::OutstationApplication::freeze_counters_range(
                    self,
                    start,
                    stop,
                    ffi::FreezeType::FreezeAndClear,
                    database as *mut _,
                )
            }
            (FreezeIndices::All, FreezeType::FreezeAtTime(timing)) => {
                let (time, interval) = timing.get_time_and_interval();
                ffi::OutstationApplication::freeze_counters_all_at_time(
                    self,
                    database as *mut _,
                    time.raw_value(),
                    interval,
                )
            }
            (FreezeIndices::Range(start, stop), FreezeType::FreezeAtTime(timing)) => {
                let (time, interval) = timing.get_time_and_interval();
                ffi::OutstationApplication::freeze_counters_range_at_time(
                    self,
                    start,
                    stop,
                    database as *mut _,
                    time.raw_value(),
                    interval,
                )
            }
        };

        result
            .map(|res| res.into())
            .unwrap_or(Err(RequestError::NotSupported))
    }

    fn support_write_analog_dead_bands(&mut self) -> bool {
        ffi::OutstationApplication::support_write_analog_dead_bands(self).unwrap_or(false)
    }

    fn begin_write_analog_dead_bands(&mut self) {
        ffi::OutstationApplication::begin_write_analog_dead_bands(self)
    }

    fn write_analog_dead_band(&mut self, index: u16, dead_band: f64) {
        ffi::OutstationApplication::write_analog_dead_band(self, index, dead_band)
    }

    fn end_write_analog_dead_bands(&mut self) -> MaybeAsync<()> {
        ffi::OutstationApplication::end_write_analog_dead_bands(self);
        MaybeAsync::ready(())
    }

    fn write_device_attr(&mut self, attr: Attribute) -> MaybeAsync<bool> {
        let any = match AnyAttribute::try_from(&attr) {
            Ok(any) => any,
            Err(_) => {
                return MaybeAsync::ready(false);
            }
        };

        let (set, var, value) = FfiAttrValue::extract(any);
        match value {
            FfiAttrValue::VariationList(_, _) => {
                // we can't write a variation list
                MaybeAsync::ready(false)
            }
            FfiAttrValue::String(attr, value) => match CString::new(value) {
                Ok(str) => {
                    let attr = attr.map(|x| x.into()).unwrap_or(ffi::StringAttr::Unknown);
                    let res = self.write_string_attr(set.value(), var, attr, &str);
                    MaybeAsync::ready(res.unwrap_or(false))
                }
                Err(err) => {
                    tracing::warn!("Cannot convert string attribute to CString: {err}");
                    MaybeAsync::ready(false)
                }
            },
            FfiAttrValue::Float(attr, value) => {
                let attr = attr.map(|x| x.into()).unwrap_or(ffi::FloatAttr::Unknown);
                let res = match value {
                    FloatType::F32(value) => self.write_float_attr(set.value(), var, attr, value),
                    FloatType::F64(value) => self.write_double_attr(set.value(), var, attr, value),
                };
                MaybeAsync::ready(res.unwrap_or(false))
            }
            FfiAttrValue::UInt(attr, value) => {
                let attr = attr.map(|x| x.into()).unwrap_or(ffi::UintAttr::Unknown);
                let res = self
                    .write_uint_attr(set.value(), var, attr, value)
                    .unwrap_or(false);
                MaybeAsync::ready(res)
            }
            FfiAttrValue::Int(value) => {
                let res = self
                    .write_int_attr(set.value(), var, ffi::IntAttr::Unknown, value)
                    .unwrap_or(false);
                MaybeAsync::ready(res)
            }
            FfiAttrValue::Bool(_, _) => {
                // none of the bool attributes are writable so we ignore this
                MaybeAsync::ready(false)
            }
            FfiAttrValue::OctetString(attr, value) => {
                let attr = attr
                    .map(|x| x.into())
                    .unwrap_or(ffi::OctetStringAttr::Unknown);
                let mut iter = crate::ByteIterator::new(value);
                let res = self
                    .write_octet_string_attr(set.value(), var, attr, &mut iter)
                    .unwrap_or(false);
                MaybeAsync::ready(res)
            }
            FfiAttrValue::BitString(value) => {
                let mut iter = crate::ByteIterator::new(value);
                let res = self
                    .write_bit_string_attr(set.value(), var, ffi::BitStringAttr::Unknown, &mut iter)
                    .unwrap_or(false);
                MaybeAsync::ready(res)
            }
            FfiAttrValue::DNP3Time(attr, value) => {
                let attr = attr.map(|x| x.into()).unwrap_or(ffi::TimeAttr::Unknown);
                let res = self
                    .write_time_attr(set.value(), var, attr, value.raw_value())
                    .unwrap_or(false);
                MaybeAsync::ready(res)
            }
        }
    }

    fn begin_confirm(&mut self) {
        ffi::OutstationApplication::begin_confirm(self)
    }

    fn event_cleared(&mut self, id: u64) {
        ffi::OutstationApplication::event_cleared(self, id);
    }

    fn end_confirm(&mut self, state: BufferState) -> MaybeAsync<()> {
        ffi::OutstationApplication::end_confirm(self, state.into());
        MaybeAsync::ready(())
    }
}

impl From<dnp3::outstation::BufferState> for ffi::BufferState {
    fn from(value: BufferState) -> Self {
        Self {
            classes: value.classes.into(),
            types: value.types.into(),
        }
    }
}

impl From<dnp3::outstation::ClassCount> for ffi::ClassCount {
    fn from(value: ClassCount) -> Self {
        Self {
            num_class_1: value.num_class_1 as u32,
            num_class_2: value.num_class_2 as u32,
            num_class_3: value.num_class_3 as u32,
        }
    }
}

impl From<dnp3::outstation::TypeCount> for ffi::TypeCount {
    fn from(value: TypeCount) -> Self {
        Self {
            num_binary_input: value.num_binary_input as u32,
            num_double_bit_binary_input: value.num_double_bit_binary_input as u32,
            num_binary_output_status: value.num_binary_output_status as u32,
            num_counter: value.num_counter as u32,
            num_frozen_counter: value.num_frozen_counter as u32,
            num_analog: value.num_analog as u32,
            num_analog_output_status: value.num_analog_output_status as u32,
            num_octet_string: value.num_octet_string as u32,
        }
    }
}

impl From<ffi::ApplicationIin> for ApplicationIin {
    fn from(from: ffi::ApplicationIin) -> Self {
        ApplicationIin {
            need_time: from.need_time(),
            local_control: from.local_control(),
            device_trouble: from.device_trouble(),
            config_corrupt: from.config_corrupt(),
        }
    }
}

impl From<ffi::RestartDelay> for Option<RestartDelay> {
    fn from(from: ffi::RestartDelay) -> Self {
        match from.restart_type() {
            ffi::RestartDelayType::NotSupported => None,
            ffi::RestartDelayType::Seconds => Some(RestartDelay::Seconds(from.value())),
            ffi::RestartDelayType::MilliSeconds => Some(RestartDelay::Milliseconds(from.value())),
        }
    }
}

impl From<ffi::WriteTimeResult> for Result<(), RequestError> {
    fn from(from: ffi::WriteTimeResult) -> Self {
        match from {
            ffi::WriteTimeResult::Ok => Ok(()),
            ffi::WriteTimeResult::ParameterError => Err(RequestError::ParameterError),
            ffi::WriteTimeResult::NotSupported => Err(RequestError::NotSupported),
        }
    }
}

impl From<ffi::FreezeResult> for Result<(), RequestError> {
    fn from(from: ffi::FreezeResult) -> Self {
        match from {
            ffi::FreezeResult::Ok => Ok(()),
            ffi::FreezeResult::ParameterError => Err(RequestError::ParameterError),
            ffi::FreezeResult::NotSupported => Err(RequestError::NotSupported),
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

    fn end_fragment(&mut self, database: &mut DatabaseHandle) -> MaybeAsync<()> {
        ffi::ControlHandler::end_fragment(self, database as *mut _);
        MaybeAsync::ready(())
    }
}

impl ControlSupport<Group12Var1> for ffi::ControlHandler {
    fn select(
        &mut self,
        control: Group12Var1,
        index: u16,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        ffi::ControlHandler::select_g12v1(self, control.into(), index, database as *mut _)
            .map(|e| e.into())
            .unwrap_or(CommandStatus::NotSupported)
    }

    fn operate(
        &mut self,
        control: Group12Var1,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
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
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        ffi::ControlHandler::select_g41v1(self, control.value, index, database as *mut _)
            .map(|e| e.into())
            .unwrap_or(CommandStatus::NotSupported)
    }

    fn operate(
        &mut self,
        control: Group41Var1,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
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
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        ffi::ControlHandler::select_g41v2(self, control.value, index, database as *mut _)
            .map(|e| e.into())
            .unwrap_or(CommandStatus::NotSupported)
    }

    fn operate(
        &mut self,
        control: Group41Var2,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
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
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        ffi::ControlHandler::select_g41v3(self, control.value, index, database as *mut _)
            .map(|e| e.into())
            .unwrap_or(CommandStatus::NotSupported)
    }

    fn operate(
        &mut self,
        control: Group41Var3,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
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
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        ffi::ControlHandler::select_g41v4(self, control.value, index, database as *mut _)
            .map(|e| e.into())
            .unwrap_or(CommandStatus::NotSupported)
    }

    fn operate(
        &mut self,
        control: Group41Var4,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
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
            control_field: from.control.into(),
            function: from.function.into(),
        }
        .into()
    }
}

impl From<ControlField> for ffi::ControlField {
    fn from(from: ControlField) -> Self {
        ffi::ControlFieldFields {
            fir: from.fir,
            fin: from.fin,
            con: from.con,
            uns: from.uns,
            seq: from.seq.value(),
        }
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

impl From<Group12Var1> for ffi::Group12Var1 {
    fn from(from: Group12Var1) -> Self {
        ffi::Group12Var1Fields {
            code: from.code.into(),
            count: from.count,
            on_time: from.on_time,
            off_time: from.off_time,
        }
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
            TripCloseCode::Unknown(_) => Self::Nul,
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
            OpType::Unknown(_) => Self::Nul,
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
            ffi::CommandStatus::Unknown => Self::Unknown(0),
        }
    }
}
