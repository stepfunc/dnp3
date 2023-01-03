use std::ffi::CString;
use dnp3::app::measurement::*;
use dnp3::app::*;
use dnp3::app::{ResponseFunction, ResponseHeader};
use dnp3::master::{
    AssociationHandler, AssociationInformation, HeaderInfo, ReadHandler, ReadType, TaskError,
    TaskType,
};

use crate::ffi;

impl AssociationHandler for ffi::AssociationHandler {
    fn get_current_time(&self) -> Option<Timestamp> {
        if let Some(time) = self.get_current_time() {
            time.into()
        } else {
            None
        }
    }
}

impl AssociationInformation for ffi::AssociationInformation {
    fn task_start(&mut self, task_type: TaskType, fc: FunctionCode, seq: Sequence) {
        ffi::AssociationInformation::task_start(self, task_type.into(), fc.into(), seq.value());
    }

    fn task_success(&mut self, task_type: TaskType, fc: FunctionCode, seq: Sequence) {
        ffi::AssociationInformation::task_success(self, task_type.into(), fc.into(), seq.value());
    }

    fn task_fail(&mut self, task_type: TaskType, error: TaskError) {
        ffi::AssociationInformation::task_fail(self, task_type.into(), error.into());
    }

    fn unsolicited_response(&mut self, is_duplicate: bool, seq: Sequence) {
        ffi::AssociationInformation::unsolicited_response(self, is_duplicate, seq.value());
    }
}

impl ReadHandler for ffi::ReadHandler {
    fn begin_fragment(&mut self, read_type: ReadType, header: ResponseHeader) -> MaybeAsync<()> {
        ffi::ReadHandler::begin_fragment(self, read_type.into(), header.into());
        MaybeAsync::ready(())
    }

    fn end_fragment(&mut self, read_type: ReadType, header: ResponseHeader) -> MaybeAsync<()> {
        ffi::ReadHandler::end_fragment(self, read_type.into(), header.into());
        MaybeAsync::ready(())
    }

    fn handle_binary_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryInput, u16)>,
    ) {
        let info = info.into();
        let mut iterator = BinaryInputIterator::new(iter);
        ffi::ReadHandler::handle_binary_input(self, info, &mut iterator as *mut _);
    }

    fn handle_double_bit_binary_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (DoubleBitBinaryInput, u16)>,
    ) {
        let info = info.into();
        let mut iterator = DoubleBitBinaryInputIterator::new(iter);
        ffi::ReadHandler::handle_double_bit_binary_input(self, info, &mut iterator as *mut _);
    }

    fn handle_binary_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryOutputStatus, u16)>,
    ) {
        let info = info.into();
        let mut iterator = BinaryOutputStatusIterator::new(iter);
        ffi::ReadHandler::handle_binary_output_status(self, info, &mut iterator as *mut _);
    }

    fn handle_counter(&mut self, info: HeaderInfo, iter: &mut dyn Iterator<Item = (Counter, u16)>) {
        let info = info.into();
        let mut iterator = CounterIterator::new(iter);
        ffi::ReadHandler::handle_counter(self, info, &mut iterator as *mut _);
    }

    fn handle_frozen_counter(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (FrozenCounter, u16)>,
    ) {
        let info = info.into();
        let mut iterator = FrozenCounterIterator::new(iter);
        ffi::ReadHandler::handle_frozen_counter(self, info, &mut iterator);
    }

    fn handle_analog_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogInput, u16)>,
    ) {
        let info = info.into();
        let mut iterator = AnalogInputIterator::new(iter);
        ffi::ReadHandler::handle_analog_input(self, info, &mut iterator);
    }

    fn handle_frozen_analog_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (FrozenAnalogInput, u16)>,
    ) {
        let info = info.into();
        let mut iterator = FrozenAnalogInputIterator::new(iter);
        ffi::ReadHandler::handle_frozen_analog_input(self, info, &mut iterator);
    }

    fn handle_analog_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogOutputStatus, u16)>,
    ) {
        let info = info.into();
        let mut iterator = AnalogOutputStatusIterator::new(iter);
        ffi::ReadHandler::handle_analog_output_status(self, info, &mut iterator);
    }

    fn handle_octet_string<'a>(
        &mut self,
        info: HeaderInfo,
        iter: &'a mut dyn Iterator<Item = (&'a [u8], u16)>,
    ) {
        let info = info.into();
        let mut iterator = OctetStringIterator::new(iter);
        ffi::ReadHandler::handle_octet_string(self, info, &mut iterator);
    }

    fn handle_device_attribute(&mut self, info: HeaderInfo, attr: AnyAttribute) {
        let info = info.into();
        match attr {
            AnyAttribute::Other(x) => {
                match x.value {
                    AttrValue::VisibleString(v) => {
                        let string = match CString::new(v) {
                            Ok(x) => x,
                            Err(err) => {
                                tracing::warn!("Attribute is not a valid C-string: {}", err);
                                return;
                            }
                        };
                        ffi::ReadHandler::handle_string_attr(self, info, ffi::StringAttr::Unknown,x.set.value(), x.variation,string.as_ref());
                    }
                    AttrValue::UnsignedInt(_) => {}
                    AttrValue::SignedInt(_) => {}
                    AttrValue::FloatingPoint(_) => {}
                    AttrValue::OctetString(_) => {}
                    AttrValue::Dnp3Time(_) => {}
                    AttrValue::BitString(_) => {}
                    AttrValue::AttrList(_) => {}
                }
            }
            AnyAttribute::Known(x) => {
                match x {
                    KnownAttribute::AttributeList(_) => {}
                    KnownAttribute::String(x, v) => {
                        let string = match CString::new(v) {
                            Ok(x) => x,
                            Err(err) => {
                                tracing::warn!("Attribute is not a valid C-string: {}", err);
                                return;
                            }
                        };
                        ffi::ReadHandler::handle_string_attr(self, info, x.into(),0, x.variation(),string.as_ref());
                    }
                    KnownAttribute::Float(_, _) => {}
                    KnownAttribute::UInt(_, _) => {}
                    KnownAttribute::Bool(_, _) => {}
                    KnownAttribute::OctetString(_, _) => {}
                    KnownAttribute::DNP3Time(_, _) => {}
                }
            }
        }
    }
}

impl From<dnp3::app::StringAttr> for ffi::StringAttr {
    fn from(value: StringAttr) -> Self {
        match value {
            StringAttr::ConfigId => Self::ConfigId,
            StringAttr::ConfigVersion => Self::ConfigVersion,
            StringAttr::ConfigDigestAlgorithm => Self::ConfigDigestAlgorithm,
            StringAttr::MasterResourceId => Self::MasterResourceId,
            StringAttr::UserAssignedSecondaryOperatorName => Self::UserAssignedSecondaryOperatorName,
            StringAttr::UserAssignedPrimaryOperatorName => Self::UserAssignedPrimaryOperatorName,
            StringAttr::UserAssignedSystemName => Self::UserAssignedSystemName,
            StringAttr::UserSpecificAttributes => Self::UserSpecificAttributes,
            StringAttr::DeviceManufacturerSoftwareVersion => Self::DeviceManufacturerSoftwareVersion,
            StringAttr::DeviceManufacturerHardwareVersion => Self::DeviceManufacturerHardwareVersion,
            StringAttr::UserAssignedOwnerName => Self::UserAssignedOwnerName,
            StringAttr::UserAssignedLocation => Self::UserAssignedLocation,
            StringAttr::UserAssignedId => Self::UserAssignedId,
            StringAttr::UserAssignedDeviceName => Self::UserAssignedDeviceName,
            StringAttr::DeviceSerialNumber => Self::DeviceSerialNumber,
            StringAttr::DeviceSubsetAndConformance => Self::DeviceSubsetAndConformance,
            StringAttr::ProductNameAndModel => Self::ProductNameAndModel,
            StringAttr::DeviceManufacturersName => Self::DeviceManufacturersName,
        }
    }
}

impl From<ReadType> for ffi::ReadType {
    fn from(x: ReadType) -> Self {
        match x {
            ReadType::Unsolicited => ffi::ReadType::Unsolicited,
            ReadType::StartupIntegrity => ffi::ReadType::StartupIntegrity,
            ReadType::PeriodicPoll => ffi::ReadType::PeriodicPoll,
            ReadType::SinglePoll => ffi::ReadType::SinglePoll,
        }
    }
}

impl From<Iin1> for ffi::Iin1 {
    fn from(x: Iin1) -> Self {
        Self {
            broadcast: x.get_broadcast(),
            class_1_events: x.get_class_1_events(),
            class_2_events: x.get_class_2_events(),
            class_3_events: x.get_class_3_events(),
            device_restart: x.get_device_restart(),
            device_trouble: x.get_device_trouble(),
            local_control: x.get_local_control(),
            need_time: x.get_need_time(),
        }
    }
}

impl From<Iin2> for ffi::Iin2 {
    fn from(x: Iin2) -> Self {
        Self {
            no_func_code_support: x.get_no_func_code_support(),
            object_unknown: x.get_object_unknown(),
            parameter_error: x.get_parameter_error(),
            event_buffer_overflow: x.get_event_buffer_overflow(),
            already_executing: x.get_already_executing(),
            config_corrupt: x.get_config_corrupt(),
            reserved_2: x.get_reserved_2(),
            reserved_1: x.get_reserved_1(),
        }
    }
}

impl From<ResponseHeader> for ffi::ResponseHeader {
    fn from(header: ResponseHeader) -> ffi::ResponseHeader {
        ffi::ResponseHeaderFields {
            control_field: ffi::ControlField {
                fir: header.control.fir,
                fin: header.control.fin,
                con: header.control.con,
                uns: header.control.uns,
                seq: header.control.seq.value(),
            },
            func: match header.function {
                ResponseFunction::Response => ffi::ResponseFunction::Response,
                ResponseFunction::UnsolicitedResponse => ffi::ResponseFunction::UnsolicitedResponse,
            },
            iin: ffi::Iin {
                iin1: header.iin.iin1.into(),
                iin2: header.iin.iin2.into(),
            },
        }
        .into()
    }
}

impl From<HeaderInfo> for ffi::HeaderInfo {
    fn from(info: HeaderInfo) -> ffi::HeaderInfo {
        ffi::HeaderInfoFields {
            variation: info.variation.into(),
            qualifier: match info.qualifier {
                QualifierCode::Range8 => ffi::QualifierCode::Range8,
                QualifierCode::Range16 => ffi::QualifierCode::Range16,
                QualifierCode::AllObjects => ffi::QualifierCode::AllObjects,
                QualifierCode::Count8 => ffi::QualifierCode::Count8,
                QualifierCode::Count16 => ffi::QualifierCode::Count16,
                QualifierCode::CountAndPrefix8 => ffi::QualifierCode::CountAndPrefix8,
                QualifierCode::CountAndPrefix16 => ffi::QualifierCode::CountAndPrefix16,
                QualifierCode::FreeFormat16 => ffi::QualifierCode::FreeFormat16,
            },
            is_event: info.is_event,
            has_flags: info.has_flags,
        }
        .into()
    }
}

impl From<TaskType> for ffi::TaskType {
    fn from(from: TaskType) -> Self {
        match from {
            TaskType::UserRead => ffi::TaskType::UserRead,
            TaskType::PeriodicPoll => ffi::TaskType::PeriodicPoll,
            TaskType::StartupIntegrity => ffi::TaskType::StartupIntegrity,
            TaskType::AutoEventScan => ffi::TaskType::AutoEventScan,
            TaskType::Command => ffi::TaskType::Command,
            TaskType::ClearRestartBit => ffi::TaskType::ClearRestartBit,
            TaskType::EnableUnsolicited => ffi::TaskType::EnableUnsolicited,
            TaskType::DisableUnsolicited => ffi::TaskType::DisableUnsolicited,
            TaskType::TimeSync => ffi::TaskType::TimeSync,
            TaskType::Restart => ffi::TaskType::Restart,
            TaskType::WriteDeadBands => ffi::TaskType::WriteDeadBands,
            TaskType::GenericEmptyResponse(_) => ffi::TaskType::GenericEmptyResponse,
        }
    }
}

macro_rules! implement_iterator {
    ($it_name:ident, $ffi_func_name:ident, $lib_type:ty, $ffi_type:ty) => {
        pub struct $it_name<'a> {
            inner: &'a mut dyn Iterator<Item = ($lib_type, u16)>,
            next: Option<$ffi_type>,
        }

        impl<'a> $it_name<'a> {
            fn new(inner: &'a mut dyn Iterator<Item = ($lib_type, u16)>) -> Self {
                Self { inner, next: None }
            }

            fn next(&mut self) {
                self.next = self
                    .inner
                    .next()
                    .map(|(value, idx)| <$ffi_type>::new(idx, value))
            }
        }

        pub unsafe fn $ffi_func_name(it: *mut $it_name) -> Option<&$ffi_type> {
            let it = it.as_mut();
            it.map(|it| {
                it.next();
                it.next.as_ref()
            })
            .flatten()
        }
    };
}

implement_iterator!(
    BinaryInputIterator,
    binary_input_iterator_next,
    BinaryInput,
    ffi::BinaryInput
);
implement_iterator!(
    DoubleBitBinaryInputIterator,
    double_bit_binary_input_iterator_next,
    DoubleBitBinaryInput,
    ffi::DoubleBitBinaryInput
);
implement_iterator!(
    BinaryOutputStatusIterator,
    binary_output_status_iterator_next,
    BinaryOutputStatus,
    ffi::BinaryOutputStatus
);
implement_iterator!(
    CounterIterator,
    counter_iterator_next,
    Counter,
    ffi::Counter
);
implement_iterator!(
    FrozenCounterIterator,
    frozen_counter_iterator_next,
    FrozenCounter,
    ffi::FrozenCounter
);
implement_iterator!(
    AnalogInputIterator,
    analog_input_iterator_next,
    AnalogInput,
    ffi::AnalogInput
);
implement_iterator!(
    FrozenAnalogInputIterator,
    frozen_analog_input_iterator_next,
    FrozenAnalogInput,
    ffi::FrozenAnalogInput
);
implement_iterator!(
    AnalogOutputStatusIterator,
    analog_output_status_iterator_next,
    AnalogOutputStatus,
    ffi::AnalogOutputStatus
);

impl ffi::BinaryInput {
    pub(crate) fn new(idx: u16, value: BinaryInput) -> Self {
        Self {
            index: idx,
            value: value.value,
            flags: value.flags.into(),
            time: value.time.into(),
        }
    }
}

impl ffi::DoubleBitBinaryInput {
    pub(crate) fn new(idx: u16, value: DoubleBitBinaryInput) -> Self {
        ffi::DoubleBitBinaryInputFields {
            index: idx,
            value: match value.value {
                DoubleBit::Intermediate => ffi::DoubleBit::Intermediate,
                DoubleBit::DeterminedOff => ffi::DoubleBit::DeterminedOff,
                DoubleBit::DeterminedOn => ffi::DoubleBit::DeterminedOn,
                DoubleBit::Indeterminate => ffi::DoubleBit::Indeterminate,
            },
            flags: value.flags.into(),
            time: value.time.into(),
        }
        .into()
    }
}

impl ffi::BinaryOutputStatus {
    pub(crate) fn new(idx: u16, value: BinaryOutputStatus) -> Self {
        Self {
            index: idx,
            value: value.value,
            flags: value.flags.into(),
            time: value.time.into(),
        }
    }
}

impl ffi::Counter {
    pub(crate) fn new(idx: u16, value: Counter) -> Self {
        Self {
            index: idx,
            value: value.value,
            flags: value.flags.into(),
            time: value.time.into(),
        }
    }
}

impl ffi::FrozenCounter {
    pub(crate) fn new(idx: u16, value: FrozenCounter) -> Self {
        Self {
            index: idx,
            value: value.value,
            flags: value.flags.into(),
            time: value.time.into(),
        }
    }
}

impl ffi::AnalogInput {
    pub(crate) fn new(idx: u16, value: AnalogInput) -> Self {
        Self {
            index: idx,
            value: value.value,
            flags: value.flags.into(),
            time: value.time.into(),
        }
    }
}

impl ffi::FrozenAnalogInput {
    pub(crate) fn new(idx: u16, value: FrozenAnalogInput) -> Self {
        Self {
            index: idx,
            value: value.value,
            flags: value.flags.into(),
            time: value.time.into(),
        }
    }
}

impl ffi::AnalogOutputStatus {
    pub(crate) fn new(idx: u16, value: AnalogOutputStatus) -> Self {
        Self {
            index: idx,
            value: value.value,
            flags: value.flags.into(),
            time: value.time.into(),
        }
    }
}

pub struct OctetStringIterator<'a> {
    inner: &'a mut dyn Iterator<Item = (&'a [u8], u16)>,
    next: Option<ffi::OctetString<'a>>,
    current_byte_it: Option<ByteIterator<'a>>,
}

impl<'a> OctetStringIterator<'a> {
    fn new(inner: &'a mut dyn Iterator<Item = (&'a [u8], u16)>) -> Self {
        Self {
            inner,
            next: None,
            current_byte_it: None,
        }
    }

    fn next(&mut self) {
        if let Some((bytes, idx)) = self.inner.next() {
            self.current_byte_it = None;
            let byte_it_ref = self.current_byte_it.get_or_insert(ByteIterator::new(bytes));
            self.next = Some(ffi::OctetString::new(idx, byte_it_ref));
        } else {
            self.next = None;
            self.current_byte_it = None;
        }
    }
}

pub unsafe fn octet_string_iterator_next(
    it: *mut OctetStringIterator,
) -> Option<&ffi::OctetString> {
    let it = it.as_mut();
    it.and_then(|it| {
        it.next();
        it.next.as_ref()
    })
}

impl<'a> ffi::OctetString<'a> {
    fn new(idx: u16, it: &mut ByteIterator<'a>) -> Self {
        Self {
            index: idx,
            value: it as *mut _,
        }
    }
}

pub struct ByteIterator<'a> {
    inner: std::slice::Iter<'a, u8>,
    next: Option<u8>,
}

impl<'a> ByteIterator<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            inner: bytes.iter(),
            next: None,
        }
    }

    fn next(&mut self) {
        self.next = self.inner.next().copied()
    }
}

pub unsafe fn byte_iterator_next(it: *mut ByteIterator) -> *const u8 {
    let it = it.as_mut();
    match it {
        None => std::ptr::null(),
        Some(x) => {
            x.next();
            match &x.next {
                None => std::ptr::null(),
                Some(x) => x,
            }
        }
    }
}

impl From<Flags> for ffi::Flags {
    fn from(flags: Flags) -> ffi::Flags {
        ffi::Flags { value: flags.value }
    }
}

impl From<Option<Time>> for ffi::Timestamp {
    fn from(time: Option<Time>) -> ffi::Timestamp {
        ffi::TimestampFields {
            value: match time {
                Some(t) => t.timestamp().raw_value(),
                None => 0,
            },
            quality: match time {
                Some(Time::Synchronized(_)) => ffi::TimeQuality::SynchronizedTime,
                Some(Time::Unsynchronized(_)) => ffi::TimeQuality::UnsynchronizedTime,
                None => ffi::TimeQuality::InvalidTime,
            },
        }
        .into()
    }
}
