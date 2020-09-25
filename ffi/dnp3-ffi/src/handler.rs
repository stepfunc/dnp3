use crate::ffi;
use dnp3::app::enums::QualifierCode;
use dnp3::app::flags::Flags;
use dnp3::app::header::{ResponseFunction, ResponseHeader, IIN1, IIN2};
use dnp3::app::measurement::*;
use dnp3::app::parse::bytes::Bytes;
use dnp3::app::types::DoubleBit;
use dnp3::master::handle::{HeaderInfo, ReadHandler};

impl ReadHandler for ffi::ReadHandler {
    fn begin_fragment(&mut self, header: ResponseHeader) {
        ffi::ReadHandler::begin_fragment(self, header.into());
    }

    fn end_fragment(&mut self, header: ResponseHeader) {
        ffi::ReadHandler::end_fragment(self, header.into());
    }

    fn handle_binary(&mut self, info: HeaderInfo, iter: &mut dyn Iterator<Item = (Binary, u16)>) {
        let info = info.into();
        let mut iterator = BinaryIterator::new(iter);
        ffi::ReadHandler::handle_binary(self, info, &mut iterator as *mut _);
    }

    fn handle_double_bit_binary(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (DoubleBitBinary, u16)>,
    ) {
        let info = info.into();
        let mut iterator = DoubleBitBinaryIterator::new(iter);
        ffi::ReadHandler::handle_double_bit_binary(self, info, &mut iterator as *mut _);
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

    fn handle_analog(&mut self, info: HeaderInfo, iter: &mut dyn Iterator<Item = (Analog, u16)>) {
        let info = info.into();
        let mut iterator = AnalogIterator::new(iter);
        ffi::ReadHandler::handle_analog(self, info, &mut iterator);
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
        iter: &'a mut dyn Iterator<Item = (Bytes<'a>, u16)>,
    ) {
        let info = info.into();
        let mut iterator = OctetStringIterator::new(iter);
        ffi::ReadHandler::handle_octet_string(self, info, &mut iterator);
    }
}

impl From<ResponseHeader> for ffi::ResponseHeader {
    fn from(header: ResponseHeader) -> ffi::ResponseHeader {
        ffi::ResponseHeaderFields {
            control: ffi::Control {
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
            iin: ffi::IIN {
                iin1: ffi::IIN1 {
                    value: header.iin.iin1.value,
                },
                iin2: ffi::IIN2 {
                    value: header.iin.iin2.value,
                },
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
        }
        .into()
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

implement_iterator!(BinaryIterator, binary_next, Binary, ffi::Binary);
implement_iterator!(
    DoubleBitBinaryIterator,
    doublebitbinary_next,
    DoubleBitBinary,
    ffi::DoubleBitBinary
);
implement_iterator!(
    BinaryOutputStatusIterator,
    binaryoutputstatus_next,
    BinaryOutputStatus,
    ffi::BinaryOutputStatus
);
implement_iterator!(CounterIterator, counter_next, Counter, ffi::Counter);
implement_iterator!(
    FrozenCounterIterator,
    frozencounter_next,
    FrozenCounter,
    ffi::FrozenCounter
);
implement_iterator!(AnalogIterator, analog_next, Analog, ffi::Analog);
implement_iterator!(
    AnalogOutputStatusIterator,
    analogoutputstatus_next,
    AnalogOutputStatus,
    ffi::AnalogOutputStatus
);

impl ffi::Binary {
    fn new(idx: u16, value: Binary) -> Self {
        Self {
            index: idx,
            value: value.value,
            flags: value.flags.into(),
            time: value.time.into(),
        }
    }
}

impl ffi::DoubleBitBinary {
    fn new(idx: u16, value: DoubleBitBinary) -> Self {
        ffi::DoubleBitBinaryFields {
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
    fn new(idx: u16, value: BinaryOutputStatus) -> Self {
        Self {
            index: idx,
            value: value.value,
            flags: value.flags.into(),
            time: value.time.into(),
        }
    }
}

impl ffi::Counter {
    fn new(idx: u16, value: Counter) -> Self {
        Self {
            index: idx,
            value: value.value,
            flags: value.flags.into(),
            time: value.time.into(),
        }
    }
}

impl ffi::FrozenCounter {
    fn new(idx: u16, value: FrozenCounter) -> Self {
        Self {
            index: idx,
            value: value.value,
            flags: value.flags.into(),
            time: value.time.into(),
        }
    }
}

impl ffi::Analog {
    fn new(idx: u16, value: Analog) -> Self {
        Self {
            index: idx,
            value: value.value,
            flags: value.flags.into(),
            time: value.time.into(),
        }
    }
}

impl ffi::AnalogOutputStatus {
    fn new(idx: u16, value: AnalogOutputStatus) -> Self {
        Self {
            index: idx,
            value: value.value,
            flags: value.flags.into(),
            time: value.time.into(),
        }
    }
}

pub struct OctetStringIterator<'a> {
    inner: &'a mut dyn Iterator<Item = (Bytes<'a>, u16)>,
    next: Option<ffi::OctetString<'a>>,
    current_byte_it: Option<ByteIterator<'a>>,
}

impl<'a> OctetStringIterator<'a> {
    fn new(inner: &'a mut dyn Iterator<Item = (Bytes<'a>, u16)>) -> Self {
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

pub unsafe fn octetstring_next(it: *mut OctetStringIterator) -> Option<&ffi::OctetString> {
    let it = it.as_mut();
    it.map(|it| {
        it.next();
        it.next.as_ref()
    })
    .flatten()
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
    next: Option<ffi::Byte>,
}

impl<'a> ByteIterator<'a> {
    fn new(bytes: Bytes<'a>) -> Self {
        Self {
            inner: bytes.value.iter(),
            next: None,
        }
    }

    fn next(&mut self) {
        self.next = self.inner.next().map(|value| ffi::Byte::new(*value))
    }
}

pub unsafe fn byte_next(it: *mut ByteIterator) -> Option<&ffi::Byte> {
    let it = it.as_mut();
    it.map(|it| {
        it.next();
        it.next.as_ref()
    })
    .flatten()
}

impl ffi::Byte {
    fn new(value: u8) -> Self {
        Self { value }
    }
}

impl From<Flags> for ffi::Flags {
    fn from(flags: Flags) -> ffi::Flags {
        ffi::Flags { value: flags.value }
    }
}

pub unsafe fn iin1_is_set(iin1: Option<&ffi::IIN1>, flag: ffi::IIN1Flag) -> bool {
    if let Some(iin1) = iin1 {
        let iin1 = IIN1::new(iin1.value);
        match flag {
            ffi::IIN1Flag::Broadcast => iin1.get_broadcast(),
            ffi::IIN1Flag::Class1Events => iin1.get_class_1_events(),
            ffi::IIN1Flag::Class2Events => iin1.get_class_2_events(),
            ffi::IIN1Flag::Class3Events => iin1.get_class_3_events(),
            ffi::IIN1Flag::NeedTime => iin1.get_need_time(),
            ffi::IIN1Flag::LocalControl => iin1.get_local_control(),
            ffi::IIN1Flag::DeviceTrouble => iin1.get_device_trouble(),
            ffi::IIN1Flag::DeviceRestart => iin1.get_device_restart(),
        }
    } else {
        false
    }
}

pub unsafe fn iin2_is_set(iin2: Option<&ffi::IIN2>, flag: ffi::IIN2Flag) -> bool {
    if let Some(iin1) = iin2 {
        let iin1 = IIN2::new(iin1.value);
        match flag {
            ffi::IIN2Flag::NoFuncCodeSupport => iin1.get_no_func_code_support(),
            ffi::IIN2Flag::ObjectUnknown => iin1.get_object_unknown(),
            ffi::IIN2Flag::ParameterError => iin1.get_parameter_error(),
            ffi::IIN2Flag::EventBufferOverflow => iin1.get_event_buffer_overflow(),
            ffi::IIN2Flag::AlreadyExecuting => iin1.get_already_executing(),
            ffi::IIN2Flag::ConfigCorrupt => iin1.get_config_corrupt(),
        }
    } else {
        false
    }
}

pub unsafe fn flags_is_set(flags: Option<&ffi::Flags>, flag: ffi::Flag) -> bool {
    if let Some(flags) = flags {
        let flags = Flags::new(flags.value);
        match flag {
            ffi::Flag::Online => flags.online(),
            ffi::Flag::Restart => flags.restart(),
            ffi::Flag::CommLost => flags.comm_lost(),
            ffi::Flag::RemoteForced => flags.remote_forced(),
            ffi::Flag::LocalForced => flags.local_forced(),
            ffi::Flag::ChatterFilter => flags.chatter_filter(),
            ffi::Flag::Rollover => flags.rollover(),
            ffi::Flag::Discontinuity => flags.discontinuity(),
            ffi::Flag::OverRange => flags.over_range(),
            ffi::Flag::ReferenceErr => flags.reference_err(),
        }
    } else {
        false
    }
}

impl From<Time> for ffi::Timestamp {
    fn from(time: Time) -> ffi::Timestamp {
        ffi::TimestampFields {
            value: match time {
                Time::Synchronized(value) => value.raw_value(),
                Time::NotSynchronized(value) => value.raw_value(),
                Time::Invalid => 0,
            },
            quality: match time {
                Time::Synchronized(_) => ffi::TimeQuality::Synchronized,
                Time::NotSynchronized(_) => ffi::TimeQuality::NotSynchronized,
                Time::Invalid => ffi::TimeQuality::Invalid,
            },
        }
        .into()
    }
}
