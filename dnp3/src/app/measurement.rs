// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::control::CommandStatus;
use crate::util::float::{f64_to_f32, f64_to_i16, f64_to_i32};
use std::time::Duration;

use crate::app::types::Timestamp;
use crate::app::variations::{
    Group102Var1, Group13Var1, Group13Var2, Group34Var1, Group34Var2, Group34Var3, Group43Var1,
    Group43Var2, Group43Var3, Group43Var4, Group43Var5, Group43Var6, Group43Var7, Group43Var8,
};
use crate::util::bit::bits;
use crate::util::bit::BitMask;
use crate::util::bit::Bitfield;

/// Enumeration modeling two stables states and an in-transit state
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DoubleBit {
    /// Transitioning between end conditions
    Intermediate,
    /// Determined to be OFF
    DeterminedOff,
    /// Determined to be ON
    DeterminedOn,
    /// Abnormal or custom condition
    Indeterminate,
}

/// A DNP3 time value that may be Synchronized or NotSynchronized
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Time {
    /// The timestamp is UTC synchronized at the remote device
    ///
    /// The master reports times read from variations with absolute time as synchronized,
    /// because those variations have no way to express time quality. Only binary and
    /// double-bit input events with absolute time (g2v2 and g4v2) must have been recorded while
    /// synchronized (TB2017-003a 5.1.3.1.2). For the others (such as g11, g13, g21, g22, g23, g32,
    /// g33, g42 and g43 with time), the outstation's clock may not have been synchronized.
    /// Relative times take the quality of their common time of occurrence (g51v1 or g51v2).
    /// Check [`HeaderInfo::variation`](crate::master::HeaderInfo::variation) to tell these apart.
    Synchronized(Timestamp),
    /// The device indicates the timestamp may be not be synchronized
    ///
    /// The master reports this only for relative times measured from an unsynchronized common
    /// time of occurrence (g51v2).
    Unsynchronized(Timestamp),
}

impl Time {
    /// test if the `Time` is synchronized
    pub fn is_synchronized(&self) -> bool {
        std::matches!(self, Self::Synchronized(_))
    }

    /// created a synchronized `Time` from a u64
    pub fn synchronized(ts: u64) -> Time {
        Self::Synchronized(Timestamp::new(ts))
    }

    /// created an unsynchronized `Time` from a u64
    pub fn unsynchronized(ts: u64) -> Time {
        Self::Unsynchronized(Timestamp::new(ts))
    }
}

/// Flags as defined in the specification where each bit has a type-specific meaning
///
/// Not every bit is used for every type (Binary, Analog, etc). Users
/// should refer to the standard to determine what flag values
/// correspond to each type.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Flags {
    /// underlying bitmask
    pub value: u8,
}

impl Flags {
    /// Object value is 'good' / 'valid' / 'nominal'
    pub const ONLINE: Flags = Flags::new(bits::BIT_0.value);
    /// Object value has not been updated since device restart
    pub const RESTART: Flags = Flags::new(bits::BIT_1.value);
    /// Object value represents the last value available before a communication failure occurred
    pub const COMM_LOST: Flags = Flags::new(bits::BIT_2.value);
    /// Object value is overridden in a downstream reporting device
    pub const REMOTE_FORCED: Flags = Flags::new(bits::BIT_3.value);
    /// object value is overridden by the device reporting this flag
    pub const LOCAL_FORCED: Flags = Flags::new(bits::BIT_4.value);
    /// Object value is changing state rapidly (device dependent meaning)
    pub const CHATTER_FILTER: Flags = Flags::new(bits::BIT_5.value);
    /// Object value exceeds the measurement range of the reported variation
    pub const OVER_RANGE: Flags = Flags::new(bits::BIT_5.value);
    /// Obsolete flag applicable only to counter object groups
    pub const ROLL_OVER: Flags = Flags::new(bits::BIT_5.value);
    /// reported counter value cannot be compared against a prior value to obtain the correct count difference
    pub const DISCONTINUITY: Flags = Flags::new(bits::BIT_6.value);
    /// Object value might not have the expected level of accuracy
    pub const REFERENCE_ERR: Flags = Flags::new(bits::BIT_6.value);

    /// Create a `Flags` struct from a `u8` bitmask
    pub const fn new(value: u8) -> Self {
        Self { value }
    }

    /// Return true if all of the flags in 'other' are set in this Flags
    pub fn is_set(&self, other: Flags) -> bool {
        (self.value & other.value) == other.value
    }
}

pub(crate) trait ToVariation<V> {
    fn to_variation(&self) -> V;
}

pub(crate) trait WireFlags {
    fn get_wire_flags(&self) -> u8;
}

impl From<Option<Time>> for Time {
    fn from(x: Option<Time>) -> Self {
        x.unwrap_or_else(|| Time::Unsynchronized(Timestamp::new(0)))
    }
}

impl From<Option<Time>> for Timestamp {
    fn from(x: Option<Time>) -> Self {
        Time::from(x).timestamp()
    }
}

impl Time {
    pub(crate) fn checked_add(self, x: u16) -> Option<Self> {
        match self {
            Time::Synchronized(ts) => ts
                .checked_add(Duration::from_millis(x.into()))
                .map(Time::Synchronized),
            Time::Unsynchronized(ts) => ts
                .checked_add(Duration::from_millis(x.into()))
                .map(Time::Unsynchronized),
        }
    }

    /// extract a `Timestamp` from a `Time` discarding synchronization information
    pub fn timestamp(&self) -> Timestamp {
        match self {
            Time::Synchronized(ts) => *ts,
            Time::Unsynchronized(ts) => *ts,
        }
    }
}

/// Measurement type corresponding to groups 1 and 2
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct BinaryInput {
    /// value of the type
    pub value: bool,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Option<Time>,
}

impl BinaryInput {
    /// construct a `BinaryInput` from its fields
    pub const fn new(value: bool, flags: Flags, time: Time) -> Self {
        Self {
            value,
            flags,
            time: Some(time),
        }
    }
}

/// Measurement type corresponding to groups 3 and 4
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct DoubleBitBinaryInput {
    /// value of the type
    pub value: DoubleBit,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Option<Time>,
}

impl DoubleBitBinaryInput {
    /// construct a `DoubleBitBinaryInput` from its fields
    pub const fn new(value: DoubleBit, flags: Flags, time: Time) -> Self {
        Self {
            value,
            flags,
            time: Some(time),
        }
    }
}

/// Measurement type corresponding to groups 10 and 11
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct BinaryOutputStatus {
    /// value of the type
    pub value: bool,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Option<Time>,
}

impl BinaryOutputStatus {
    /// construct a `BinaryOutputStatus` from its fields
    pub const fn new(value: bool, flags: Flags, time: Time) -> Self {
        Self {
            value,
            flags,
            time: Some(time),
        }
    }
}

/// Measurement type corresponding to groups 20 and 22
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Counter {
    /// value of the type
    pub value: u32,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Option<Time>,
}

impl Counter {
    /// construct a `Counter` from its fields
    pub const fn new(value: u32, flags: Flags, time: Time) -> Self {
        Self {
            value,
            flags,
            time: Some(time),
        }
    }
}

/// Measurement type corresponding to groups 21 and 23
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct FrozenCounter {
    /// value of the type
    pub value: u32,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Option<Time>,
}

impl FrozenCounter {
    /// construct a `FrozenCounter` from its fields
    pub const fn new(value: u32, flags: Flags, time: Time) -> Self {
        Self {
            value,
            flags,
            time: Some(time),
        }
    }
}

/// Measurement type corresponding to groups 30 and 32
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AnalogInput {
    /// value of the type
    pub value: f64,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Option<Time>,
}

impl AnalogInput {
    /// construct an `AnalogInput` from its fields
    pub const fn new(value: f64, flags: Flags, time: Time) -> Self {
        Self {
            value,
            flags,
            time: Some(time),
        }
    }
}

/// Measurement type corresponding to groups 30 and 32
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FrozenAnalogInput {
    /// value of the type
    pub value: f64,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Option<Time>,
}

impl FrozenAnalogInput {
    /// construct an `AnalogInput` from its fields
    pub const fn new(value: f64, flags: Flags, time: Time) -> Self {
        Self {
            value,
            flags,
            time: Some(time),
        }
    }
}

/// Event transferred from master to outstation when the outstation receives a control.
/// The primary use case of these objects are that they allow one master see that commands
/// were issued to an outstation from another master.
///
/// Maps to group 13 variations 1 and 2.
///
/// These objects are part of subset level 4 and are not commonly used.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct BinaryOutputCommandEvent {
    /// commanded state of the binary output
    ///
    /// From the spec:
    ///
    /// 0 = Latch Off / Trip / NULL, 1 = Latch On / Close. Where the
    /// commanded state is unknown, the commanded state flag shall be 0.
    pub commanded_state: bool,
    /// status from processing the command that triggered this event
    pub status: CommandStatus,
    /// associated time
    pub time: Option<Time>,
}

impl BinaryOutputCommandEvent {
    fn extract_state_and_status(value: u8) -> (bool, CommandStatus) {
        let state = (value & 0b1000_0000) != 0;
        let status = CommandStatus::from(value & 0b0111_1111);
        (state, status)
    }
}

/// Corresponds to the different ways in which an analog output value
/// may be encoded.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum AnalogCommandValue {
    /// 16-bit integer
    I16(i16),
    /// 32-bit integer
    I32(i32),
    /// single-precision floating point
    F32(f32),
    /// double-precision floating point
    F64(f64),
}

/// Event transferred from master to outstation when the outstation receives an analog output.
/// The primary use case of these objects are that they allow one master see that commands
/// were issued to an outstation from another master.
///
/// Maps to group 43 variations 1 to 8
///
/// These objects are part of subset level 4 and are not commonly used.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct AnalogOutputCommandEvent {
    /// status from processing the command that triggered this event
    pub status: CommandStatus,
    /// value in the originating analog out
    pub commanded_value: AnalogCommandValue,
    /// associated time
    pub time: Option<Time>,
}

/// Analog input dead-band received from the outstation
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AnalogInputDeadBand {
    /// Value corresponding to g34v1
    U16(u16),
    /// Value corresponding to g34v2
    U32(u32),
    /// Value corresponding to g34v3
    F32(f32),
}

/// Type corresponding to g102v1
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct UnsignedInteger {
    /// Value of the type
    pub value: u8,
}

impl From<Group34Var1> for AnalogInputDeadBand {
    fn from(value: Group34Var1) -> Self {
        Self::U16(value.value)
    }
}

impl From<Group34Var2> for AnalogInputDeadBand {
    fn from(value: Group34Var2) -> Self {
        AnalogInputDeadBand::U32(value.value)
    }
}

impl From<Group34Var3> for AnalogInputDeadBand {
    fn from(value: Group34Var3) -> Self {
        AnalogInputDeadBand::F32(value.value)
    }
}

impl From<Group13Var1> for BinaryOutputCommandEvent {
    fn from(value: Group13Var1) -> Self {
        let (commanded_state, status) = Self::extract_state_and_status(value.flags);
        Self {
            commanded_state,
            status,
            time: None,
        }
    }
}

impl From<Group13Var2> for BinaryOutputCommandEvent {
    fn from(value: Group13Var2) -> Self {
        let (commanded_state, status) = Self::extract_state_and_status(value.flags);
        Self {
            commanded_state,
            status,
            time: Some(Time::Synchronized(value.time)),
        }
    }
}

impl From<Group43Var1> for AnalogOutputCommandEvent {
    fn from(value: Group43Var1) -> Self {
        Self {
            status: value.status,
            commanded_value: AnalogCommandValue::I32(value.value),
            time: None,
        }
    }
}

impl From<Group43Var2> for AnalogOutputCommandEvent {
    fn from(value: Group43Var2) -> Self {
        Self {
            status: value.status,
            commanded_value: AnalogCommandValue::I16(value.value),
            time: None,
        }
    }
}

impl From<Group43Var3> for AnalogOutputCommandEvent {
    fn from(value: Group43Var3) -> Self {
        Self {
            status: value.status,
            commanded_value: AnalogCommandValue::I32(value.value),
            time: Some(Time::Synchronized(value.time)),
        }
    }
}

impl From<Group43Var4> for AnalogOutputCommandEvent {
    fn from(value: Group43Var4) -> Self {
        Self {
            status: value.status,
            commanded_value: AnalogCommandValue::I16(value.value),
            time: Some(Time::Synchronized(value.time)),
        }
    }
}

impl From<Group43Var5> for AnalogOutputCommandEvent {
    fn from(value: Group43Var5) -> Self {
        Self {
            status: value.status,
            commanded_value: AnalogCommandValue::F32(value.value),
            time: None,
        }
    }
}

impl From<Group43Var6> for AnalogOutputCommandEvent {
    fn from(value: Group43Var6) -> Self {
        Self {
            status: value.status,
            commanded_value: AnalogCommandValue::F64(value.value),
            time: None,
        }
    }
}

impl From<Group43Var7> for AnalogOutputCommandEvent {
    fn from(value: Group43Var7) -> Self {
        Self {
            status: value.status,
            commanded_value: AnalogCommandValue::F32(value.value),
            time: Some(Time::Synchronized(value.time)),
        }
    }
}

impl From<Group43Var8> for AnalogOutputCommandEvent {
    fn from(value: Group43Var8) -> Self {
        Self {
            status: value.status,
            commanded_value: AnalogCommandValue::F64(value.value),
            time: Some(Time::Synchronized(value.time)),
        }
    }
}

impl From<Group102Var1> for UnsignedInteger {
    fn from(obj: Group102Var1) -> Self {
        Self { value: obj.value }
    }
}

impl std::ops::BitOr<Flags> for Flags {
    type Output = Flags;

    fn bitor(self, rhs: Flags) -> Self::Output {
        Flags::new(self.value | rhs.value)
    }
}

impl std::ops::BitOrAssign<Flags> for Flags {
    fn bitor_assign(&mut self, rhs: Flags) {
        self.value |= rhs.value
    }
}

// some crate only helpers
impl Flags {
    /// test a `Flags` struct to see if the `STATE` bit is set
    pub(crate) fn state(self) -> bool {
        self.value.bit_7()
    }

    /// extract the `DoubleBit` value from a flags struct
    pub(crate) fn double_bit_state(self) -> DoubleBit {
        DoubleBit::from(self.value.bit_7(), self.value.bit_6())
    }

    pub(crate) fn with_bits_set_to(&self, mask: BitMask, value: bool) -> Flags {
        if value {
            self.with_bits_set(mask)
        } else {
            self.with_bits_cleared(mask)
        }
    }

    pub(crate) fn with_bits_cleared(&self, mask: BitMask) -> Flags {
        Flags::new(self.value & !mask.value)
    }

    pub(crate) fn with_bits_set(&self, mask: BitMask) -> Flags {
        Flags::new(self.value | mask.value)
    }

    pub(crate) fn without(&self, mask: BitMask) -> Flags {
        Flags::new(self.value & !mask.value)
    }
}

/// Writes the names of the set flags, separated by commas
struct FlagList {
    prev: bool,
}

impl FlagList {
    fn push(
        &mut self,
        is_set: bool,
        text: &'static str,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        if is_set {
            if self.prev {
                f.write_str(", ")?;
            }
            self.prev = true;
            f.write_str(text)?;
        }
        Ok(())
    }

    fn push_debug_item<T>(
        &mut self,
        name: &'static str,
        item: T,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result
    where
        T: std::fmt::Debug,
    {
        if self.prev {
            f.write_str(", ")?;
        }
        self.prev = true;
        write!(f, "{name} = {item:?}")
    }
}

/// What bits 5 to 7 of the flags mean, which depends on the measurement type
#[derive(Copy, Clone)]
enum HighFlags {
    /// the names of bits 5, 6 and 7
    Named([&'static str; 3]),
    /// bit 5 is CHATTER_FILTER and bits 6 and 7 are the double-bit state
    ChatterFilterAndState,
}

/// Displays a flags byte and the names of the bits it sets
pub(crate) struct FlagFormatter {
    flags: Flags,
    high: HighFlags,
}

impl FlagFormatter {
    fn named(value: u8, names: [&'static str; 3]) -> Self {
        Self {
            flags: Flags::new(value),
            high: HighFlags::Named(names),
        }
    }

    pub(crate) fn binary(value: u8) -> Self {
        Self::named(value, ["CHATTER_FILTER", "RESERVED(6)", "STATE"])
    }

    pub(crate) fn double_bit_binary(value: u8) -> Self {
        Self {
            flags: Flags::new(value),
            high: HighFlags::ChatterFilterAndState,
        }
    }

    pub(crate) fn binary_output_status(value: u8) -> Self {
        Self::named(value, ["RESERVED(5)", "RESERVED(6)", "STATE"])
    }

    pub(crate) fn counter(value: u8) -> Self {
        Self::named(value, ["ROLLOVER", "DISCONTINUITY", "RESERVED(7)"])
    }

    pub(crate) fn analog(value: u8) -> Self {
        Self::named(value, ["OVER_RANGE", "REFERENCE_ERR", "RESERVED(7)"])
    }
}

impl std::fmt::Display for FlagFormatter {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let flags = self.flags;
        write!(f, "0x{:02X} [", flags.value)?;
        let mut list = FlagList { prev: false };
        list.push(flags.is_set(Flags::ONLINE), "ONLINE", f)?;
        list.push(flags.is_set(Flags::RESTART), "RESTART", f)?;
        list.push(flags.is_set(Flags::COMM_LOST), "COMM_LOST", f)?;
        list.push(flags.is_set(Flags::REMOTE_FORCED), "REMOTE_FORCED", f)?;
        list.push(flags.is_set(Flags::LOCAL_FORCED), "LOCAL_FORCED", f)?;
        match self.high {
            HighFlags::Named([bit_5, bit_6, bit_7]) => {
                list.push(flags.value.bit_5(), bit_5, f)?;
                list.push(flags.value.bit_6(), bit_6, f)?;
                list.push(flags.value.bit_7(), bit_7, f)?;
            }
            HighFlags::ChatterFilterAndState => {
                list.push(flags.is_set(Flags::CHATTER_FILTER), "CHATTER_FILTER", f)?;
                list.push_debug_item("state", flags.double_bit_state(), f)?;
            }
        }
        f.write_str("]")
    }
}

pub(crate) trait AnalogConversions {
    fn get_value(&self) -> f64;
    fn get_flags(&self) -> Flags;

    /// the flags with OVER_RANGE set, for a value the variation can't represent
    fn over_range<N>(&self, value: N) -> (Flags, N) {
        (self.get_flags() | Flags::OVER_RANGE, value)
    }

    /// NaN is 0 and out of range values clamp, both with OVER_RANGE
    fn to_i16(&self) -> (Flags, i16) {
        let value = self.get_value();

        if value.is_nan() {
            return self.over_range(0);
        }

        if value < i16::MIN.into() {
            return self.over_range(i16::MIN);
        }

        if value > i16::MAX.into() {
            return self.over_range(i16::MAX);
        }

        (self.get_flags(), f64_to_i16(value))
    }

    /// NaN is 0 and out of range values clamp, both with OVER_RANGE
    fn to_i32(&self) -> (Flags, i32) {
        let value = self.get_value();

        if value.is_nan() {
            return self.over_range(0);
        }

        if value < i32::MIN.into() {
            return self.over_range(i32::MIN);
        }

        if value > i32::MAX.into() {
            return self.over_range(i32::MAX);
        }

        (self.get_flags(), f64_to_i32(value))
    }

    /// NaN and infinity are kept, and finite values past the f32 range clamp with OVER_RANGE
    fn to_f32(&self) -> (Flags, f32) {
        let value = self.get_value();

        if value.is_finite() {
            if value < f32::MIN.into() {
                return self.over_range(f32::MIN);
            }

            if value > f32::MAX.into() {
                return self.over_range(f32::MAX);
            }
        }

        (self.get_flags(), f64_to_f32(value))
    }
}

/// Measurement type corresponding to groups 40 and 42
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct AnalogOutputStatus {
    /// value of the type
    pub value: f64,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Option<Time>,
}

impl AnalogOutputStatus {
    /// construct an `AnalogOutputStatus` from its fields
    pub fn new(value: f64, flags: Flags, time: Time) -> Self {
        Self {
            value,
            flags,
            time: Some(time),
        }
    }
}

/// Octet string point type corresponding to groups 110 and 111
///
/// Octet strings can only hold from 1 to 255 octets. Zero-length
/// octet strings are prohibited by the standard.
///
/// The default value is `[0x00]`, corresponding to an empty
/// C-style string.
#[expect(
    missing_copy_implementations,
    reason = "an octet string holds 255 bytes, too many to copy implicitly"
)]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct OctetString {
    value: [u8; Self::MAX_SIZE],
    len: u8,
}

impl Default for OctetString {
    /// A single zero byte
    fn default() -> Self {
        Self {
            value: [0; Self::MAX_SIZE],
            len: 1,
        }
    }
}

impl OctetString {
    const MAX_SIZE: usize = 255;

    /// Creates a new octet string.
    ///
    /// The `value` parameter must have a length <= 255 bytes. Zero-length (empty)
    /// octet strings may be constructed and transmitted, although the standard
    /// explicitly disallows them and they are not parsed by this library by default.
    pub fn new(value: &[u8]) -> Result<Self, OctetStringLengthError> {
        let len =
            u8::try_from(value.len()).map_err(|_| OctetStringLengthError::MoreThan255Octets)?;

        let mut result = Self {
            value: [0u8; Self::MAX_SIZE],
            len,
        };
        for (dest, src) in result.value.iter_mut().zip(value) {
            *dest = *src;
        }
        Ok(result)
    }

    /// Returns the value of the octet string
    pub fn value(&self) -> &[u8] {
        self.value.get(..usize::from(self.len)).unwrap_or_default()
    }

    /// Returns the length of the octet string
    pub fn len(&self) -> u8 {
        self.len
    }

    /// True if the octet string is empty, which the standard prohibits but which may be
    /// constructed
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Allocates a new slice with the exact size of the string
    /// and copies the content to it.
    pub(crate) fn as_boxed_slice(&self) -> Box<[u8]> {
        self.value().into()
    }
}

/// Errors when creating an octet string
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OctetStringLengthError {
    /// The standard explicitly disallows zero-length octet strings; however, many implementations
    /// use them anyway. This error variant is now unused by the library as the library allows zero-length
    /// strings to be contructed and transmitted, but will not parse them by default.
    ZeroLength,
    /// Octet strings can only hold up to 255 octets.
    MoreThan255Octets,
}

/// The low 16 bits of a counter, which is how a 32-bit counter rolls over in a 16-bit variation
pub(crate) fn low_16_bits(value: u32) -> u16 {
    let [low, high, _, _] = value.to_le_bytes();
    u16::from_le_bytes([low, high])
}

/// Edge values for the analog conversions, and a model of the integer conversions that never
/// converts a float with `as`, shared by the conversion properties here and master extraction's
#[cfg(test)]
pub(crate) mod analog_model {
    use proptest::prelude::*;

    /// zero and the bounds of i16, i32 and f32
    fn limits() -> Vec<f64> {
        vec![
            0.0,
            i16::MIN.into(),
            i16::MAX.into(),
            i32::MIN.into(),
            i32::MAX.into(),
            f32::MIN.into(),
            f32::MAX.into(),
        ]
    }

    /// half an f32 step past `f32::MAX`, where rounding to f32 gives infinity
    fn past_f32_max() -> f64 {
        f64::from(f32::MAX) + 2f64.powi(103)
    }

    /// each limit, one less and one more, and the adjacent `f64` values
    fn neighbors() -> Vec<f64> {
        limits()
            .into_iter()
            .flat_map(|limit| {
                [
                    limit.next_down(),
                    limit - 1.0,
                    limit,
                    limit + 1.0,
                    limit.next_up(),
                ]
            })
            .collect()
    }

    /// the values on both sides of rounding past the f32 range, the infinities, NaN of both
    /// signs, negative zero and the smallest subnormals
    fn specials() -> Vec<f64> {
        vec![
            past_f32_max(),
            past_f32_max().next_down(),
            -past_f32_max(),
            -past_f32_max().next_down(),
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            -f64::NAN,
            -0.0,
            f64::from_bits(1),
            -f64::from_bits(1),
        ]
    }

    /// fractions, values just past the integer limits, and values far past each range
    fn fixed() -> Vec<f64> {
        vec![
            -1.9,
            0.5,
            -0.5,
            -40000.0,
            40000.0,
            -32768.5,
            32767.5,
            -2147483648.5,
            2147483647.5,
            -1e12,
            1e12,
            f64::MIN,
            f64::MAX,
        ]
    }

    /// the limits and their neighbors, the special values and the fixed edges
    pub(crate) fn edges() -> Vec<f64> {
        neighbors()
            .into_iter()
            .chain(specials())
            .chain(fixed())
            .collect()
    }

    /// every bit pattern, the edges, values near the limits, and the midpoints between adjacent
    /// f32 values, where rounding has to break a tie
    pub(crate) fn value() -> impl Strategy<Value = f64> {
        prop_oneof![
            any::<u64>().prop_map(f64::from_bits),
            -1e10f64..1e10,
            (prop::sample::select(limits()), -2.0f64..2.0)
                .prop_map(|(limit, offset)| limit + offset),
            prop::sample::select(neighbors()),
            prop::sample::select(specials()),
            prop::sample::select(fixed()),
            any::<u32>().prop_filter_map("a finite f32 below another", |bits| {
                let low = f32::from_bits(bits);
                let high = low.next_up();
                high.is_finite()
                    .then(|| (f64::from(low) + f64::from(high)) / 2.0)
            }),
        ]
    }

    /// the wire value as an `f64`, and whether it is over range, for an integer variation with
    /// the range `[min, max]`: NaN is 0, values past the range clamp, and the rest truncate toward
    /// zero
    pub(crate) fn integer(value: f64, min: f64, max: f64) -> (f64, bool) {
        if value.is_nan() {
            (0.0, true)
        } else if value < min {
            (min, true)
        } else if value > max {
            (max, true)
        } else {
            (value.trunc(), false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_16_bit_counter_keeps_the_low_16_bits() {
        assert_eq!(low_16_bits(0x0000_FFFF), 0xFFFF);
        assert_eq!(low_16_bits(0x0001_0000), 0);
        assert_eq!(low_16_bits(0x1234_5678), 0x5678);
    }

    /// Property: for an analog input, a frozen analog input and an analog output status with any
    /// value and flags, `to_i16` and `to_i32` give 0 for NaN, clamp values past the range and
    /// truncate the rest toward zero; `to_f32` keeps NaN and the infinities, clamps finite values
    /// past the f32 range, and rounds the rest to the nearest f32, ties to even, keeping the sign
    /// of zero. The flags are kept, with OVER_RANGE added exactly when the value was NaN (for the
    /// integers) or clamped.
    ///
    /// The oracle is independent: [`analog_model`] predicts the wire value as an `f64`, which
    /// every `i16`, `i32` and `f32` converts to exactly, and never converts a float with `as`; the
    /// f32 rounding check `rounds_to` has its own test. Master extraction's property in
    /// `master/write_extract_tests.rs` shares the integer model, so a mistake in it would hide
    /// from both.
    ///
    /// The generator draws every f64 bit pattern, values within 2 of each limit, the limits and
    /// their neighbors, NaN of both signs, the infinities, -0.0, subnormals, the values on both
    /// sides of rounding past `f32::MAX`, and the midpoints between adjacent f32 values, where
    /// rounding breaks a tie, with any flags byte. An exact limit is one choice among many, so
    /// `edge_values_match_the_model` runs every edge in each run. The three types are every
    /// implementation of `AnalogConversions`; which conversion each variation uses is left to
    /// master extraction's property.
    mod conversion_properties {
        use super::*;
        use proptest::prelude::*;

        fn expected_flags(flags: Flags, over_range: bool) -> Flags {
            if over_range {
                flags | Flags::OVER_RANGE
            } else {
                flags
            }
        }

        /// whether a finite `value` within the f32 range rounds to `actual`: the nearest f32, with
        /// a tie going to the even one, keeping the sign of zero
        fn rounds_to(value: f64, actual: f32) -> bool {
            if !actual.is_finite() || actual.is_sign_negative() != value.is_sign_negative() {
                return false;
            }
            // the midpoint of two adjacent f32 values is exact in f64
            let midpoint = |other: f32| (f64::from(actual) + f64::from(other)) / 2.0;
            let even = actual.to_bits() & 1 == 0;
            let within = |bound: f64, past: std::cmp::Ordering| match value.partial_cmp(&bound) {
                Some(std::cmp::Ordering::Equal) => even,
                Some(order) => order != past,
                None => false,
            };
            // past f32::MAX or f32::MIN there is no neighbor, and the value is in range
            let below = actual.next_down();
            let above = actual.next_up();
            (!below.is_finite() || within(midpoint(below), std::cmp::Ordering::Less))
                && (!above.is_finite() || within(midpoint(above), std::cmp::Ordering::Greater))
        }

        /// whether `to_f32` of `value` with `flags` may give `actual`
        fn f32_conversion_is_correct(
            value: f64,
            flags: Flags,
            (actual_flags, actual): (Flags, f32),
        ) -> bool {
            let f32_max = f64::from(f32::MAX);
            if value.is_nan() {
                actual.is_nan() && actual_flags == flags
            } else if value.is_infinite() {
                actual.is_infinite()
                    && actual.is_sign_negative() == value.is_sign_negative()
                    && actual_flags == flags
            } else if value.abs() > f32_max {
                let clamped = if value > 0.0 { f32::MAX } else { f32::MIN };
                actual.to_bits() == clamped.to_bits() && actual_flags == flags | Flags::OVER_RANGE
            } else {
                rounds_to(value, actual) && actual_flags == flags
            }
        }

        #[test]
        fn the_f32_model_rejects_incorrect_conversions() {
            let flags = Flags::ONLINE;
            let correct =
                |value: f64, actual: f32| f32_conversion_is_correct(value, flags, (flags, actual));
            assert!(correct(1.0, 1.0));
            assert!(!correct(1.0, 0.0));
            assert!(!correct(1.0, f32::NAN));
            assert!(!correct(1.0, 1.0f32.next_up()));
            assert!(!correct(1.0, 1.0f32.next_down()));
            assert!(!correct(1.0, f32::INFINITY));
            assert!(!correct(-1e-300, 0.0));
            assert!(correct(-1e-300, -0.0));
            assert!(!correct(f64::NAN, 0.0));
            assert!(!correct(f64::INFINITY, f32::MAX));
            // 1 + 2^-24 is halfway between 1, which is even, and the next f32, which is odd
            let tie = 1.0 + 2f64.powi(-24);
            assert!(correct(tie, 1.0));
            assert!(!correct(tie, 1.0f32.next_up()));
            // halfway between that odd f32 and the next even one
            let tie = 1.0 + 3.0 * 2f64.powi(-24);
            assert!(correct(tie, 1.0f32.next_up().next_up()));
            assert!(!correct(tie, 1.0f32.next_up()));
            // the flags are part of the result
            assert!(!f32_conversion_is_correct(
                1.0,
                flags,
                (flags | Flags::OVER_RANGE, 1.0)
            ));
            assert!(!f32_conversion_is_correct(
                f64::MAX,
                flags,
                (flags, f32::MAX)
            ));
        }

        fn check<T: AnalogConversions>(measurement: T, value: f64, flags: Flags) {
            let (expected, over_range) =
                analog_model::integer(value, i16::MIN.into(), i16::MAX.into());
            let (actual_flags, actual) = measurement.to_i16();
            assert_eq!(f64::from(actual), expected, "{value}");
            assert_eq!(actual_flags, expected_flags(flags, over_range), "{value}");

            let (expected, over_range) =
                analog_model::integer(value, i32::MIN.into(), i32::MAX.into());
            let (actual_flags, actual) = measurement.to_i32();
            assert_eq!(f64::from(actual), expected, "{value}");
            assert_eq!(actual_flags, expected_flags(flags, over_range), "{value}");

            let actual = measurement.to_f32();
            assert!(
                f32_conversion_is_correct(value, flags, actual),
                "{value} -> {actual:?}"
            );
        }

        /// checks each analog type that converts to the wire
        fn check_all(value: f64, flags: Flags) {
            let time = Time::unsynchronized(0);
            check(AnalogInput::new(value, flags, time), value, flags);
            check(FrozenAnalogInput::new(value, flags, time), value, flags);
            check(AnalogOutputStatus::new(value, flags, time), value, flags);
        }

        #[test]
        fn edge_values_match_the_model() {
            for value in analog_model::edges() {
                check_all(value, Flags::ONLINE);
            }
        }

        proptest! {
            #![proptest_config(crate::util::test_support::proptest_config(1024))]

            #[test]
            fn analog_conversions_match_the_model(value in analog_model::value(), flags in any::<u8>()) {
                check_all(value, Flags::new(flags));
            }
        }
    }

    #[test]
    fn octet_string_methods() {
        let octet_string = OctetString::new(&[0, 1, 2, 3, 4]).unwrap();
        assert_eq!(5, octet_string.len());
        assert_eq!(&[0, 1, 2, 3, 4], octet_string.value());
        assert_eq!(&[0, 1, 2, 3, 4], &*octet_string.as_boxed_slice());
    }

    #[test]
    fn allows_creating_zero_length_octet_string() {
        OctetString::new(&[]).unwrap();
    }

    #[test]
    fn new_octet_string_greater_size() {
        assert_eq!(
            Err(OctetStringLengthError::MoreThan255Octets),
            OctetString::new(&[0; 500])
        );
    }

    #[test]
    fn octet_string_default_value() {
        assert_eq!(&[0x00], OctetString::default().value());
    }

    #[test]
    fn flag_bit_or_works() {
        let flags = Flags::ONLINE | Flags::LOCAL_FORCED;
        assert_eq!(flags.value, 0b0001_0001);
    }

    #[test]
    fn flag_bit_or_assign_works() {
        let mut flags = Flags::ONLINE;
        flags |= Flags::LOCAL_FORCED;
        assert_eq!(flags.value, 0b0001_0001);
    }

    #[test]
    fn each_flag_family_names_its_bits_in_the_decoded_log() {
        use crate::app::variations::{
            Group10Var2, Group1Var2, Group20Var1, Group30Var1, Group3Var2,
        };

        let cases = [
            (Group1Var2 { flags: 0 }.to_string(), "flags: 0x00 []"),
            (
                Group1Var2 { flags: 0xFF }.to_string(),
                "flags: 0xFF [ONLINE, RESTART, COMM_LOST, REMOTE_FORCED, LOCAL_FORCED, CHATTER_FILTER, RESERVED(6), STATE]",
            ),
            (
                Group3Var2 { flags: 0 }.to_string(),
                "flags: 0x00 [state = Intermediate]",
            ),
            (
                Group3Var2 { flags: 0xFF }.to_string(),
                "flags: 0xFF [ONLINE, RESTART, COMM_LOST, REMOTE_FORCED, LOCAL_FORCED, CHATTER_FILTER, state = Indeterminate]",
            ),
            (Group10Var2 { flags: 0 }.to_string(), "flags: 0x00 []"),
            (
                Group10Var2 { flags: 0xFF }.to_string(),
                "flags: 0xFF [ONLINE, RESTART, COMM_LOST, REMOTE_FORCED, LOCAL_FORCED, RESERVED(5), RESERVED(6), STATE]",
            ),
            (
                Group20Var1 { flags: 0, value: 7 }.to_string(),
                "flags: 0x00 [] value: 7",
            ),
            (
                Group20Var1 { flags: 0xFF, value: 7 }.to_string(),
                "flags: 0xFF [ONLINE, RESTART, COMM_LOST, REMOTE_FORCED, LOCAL_FORCED, ROLLOVER, DISCONTINUITY, RESERVED(7)] value: 7",
            ),
            (
                Group30Var1 { flags: 0, value: -3 }.to_string(),
                "flags: 0x00 [] value: -3",
            ),
            (
                Group30Var1 { flags: 0xFF, value: -3 }.to_string(),
                "flags: 0xFF [ONLINE, RESTART, COMM_LOST, REMOTE_FORCED, LOCAL_FORCED, OVER_RANGE, REFERENCE_ERR, RESERVED(7)] value: -3",
            ),
        ];

        for (actual, expected) in cases {
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn formats_binary_flags() {
        assert_eq!(format!("{}", FlagFormatter::binary(0)), "0x00 []");
        assert_eq!(
            format!("{}", FlagFormatter::binary(0b1100_0001)),
            "0xC1 [ONLINE, RESERVED(6), STATE]"
        );
    }

    #[test]
    fn formats_double_flags() {
        assert_eq!(
            format!("{}", FlagFormatter::double_bit_binary(0)),
            "0x00 [state = Intermediate]"
        );
        assert_eq!(
            format!("{}", FlagFormatter::double_bit_binary(0b1100_0001)),
            "0xC1 [ONLINE, state = Indeterminate]"
        );
    }
}
