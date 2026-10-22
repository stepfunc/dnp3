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
    Synchronized(Timestamp),
    /// The device indicates the timestamp may be not be synchronized
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

struct FlagFormatter {
    prev: bool,
}

impl FlagFormatter {
    fn new() -> Self {
        Self { prev: false }
    }

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

    fn begin(flags: Flags, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "0x{:02X} [", flags.value)
    }

    fn end(f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("]")
    }

    fn format_binary_flags_0_to_4(
        &mut self,
        flags: Flags,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        self.push(flags.is_set(Flags::ONLINE), "ONLINE", f)?;
        self.push(flags.is_set(Flags::RESTART), "RESTART", f)?;
        self.push(flags.is_set(Flags::COMM_LOST), "COMM_LOST", f)?;
        self.push(flags.is_set(Flags::REMOTE_FORCED), "REMOTE_FORCED", f)?;
        self.push(flags.is_set(Flags::LOCAL_FORCED), "LOCAL_FORCED", f)?;
        Ok(())
    }

    fn format_binary_flags_0_to_5(
        &mut self,
        flags: Flags,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        self.format_binary_flags_0_to_4(flags, f)?;
        self.push(flags.is_set(Flags::CHATTER_FILTER), "CHATTER_FILTER", f)?;
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

pub(crate) struct BinaryFlagFormatter {
    flags: Flags,
}

impl BinaryFlagFormatter {
    pub(crate) fn new(value: u8) -> Self {
        Self {
            flags: Flags::new(value),
        }
    }
}

impl std::fmt::Display for BinaryFlagFormatter {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut formatter = FlagFormatter::new();
        FlagFormatter::begin(self.flags, f)?;
        formatter.format_binary_flags_0_to_5(self.flags, f)?;
        formatter.push(self.flags.value.bit_6(), "RESERVED(6)", f)?;
        formatter.push(self.flags.value.bit_7(), "STATE", f)?;
        FlagFormatter::end(f)
    }
}

pub(crate) struct DoubleBitBinaryFlagFormatter {
    flags: Flags,
}

impl DoubleBitBinaryFlagFormatter {
    pub(crate) fn new(value: u8) -> Self {
        Self {
            flags: Flags::new(value),
        }
    }
}

impl std::fmt::Display for DoubleBitBinaryFlagFormatter {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut formatter = FlagFormatter::new();
        FlagFormatter::begin(self.flags, f)?;
        formatter.format_binary_flags_0_to_5(self.flags, f)?;
        formatter.push_debug_item("state", self.flags.double_bit_state(), f)?;
        FlagFormatter::end(f)
    }
}

pub(crate) struct BinaryOutputStatusFlagFormatter {
    flags: Flags,
}

impl BinaryOutputStatusFlagFormatter {
    pub(crate) fn new(value: u8) -> Self {
        Self {
            flags: Flags::new(value),
        }
    }
}

impl std::fmt::Display for BinaryOutputStatusFlagFormatter {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut formatter = FlagFormatter::new();
        FlagFormatter::begin(self.flags, f)?;
        formatter.format_binary_flags_0_to_4(self.flags, f)?;
        formatter.push(self.flags.value.bit_5(), "RESERVED(5)", f)?;
        formatter.push(self.flags.value.bit_6(), "RESERVED(6)", f)?;
        formatter.push(self.flags.value.bit_7(), "STATE", f)?;
        FlagFormatter::end(f)
    }
}

pub(crate) struct CounterFlagFormatter {
    flags: Flags,
}

impl CounterFlagFormatter {
    pub(crate) fn new(value: u8) -> Self {
        Self {
            flags: Flags::new(value),
        }
    }
}

impl std::fmt::Display for CounterFlagFormatter {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut formatter = FlagFormatter::new();
        FlagFormatter::begin(self.flags, f)?;
        formatter.format_binary_flags_0_to_4(self.flags, f)?;
        formatter.push(self.flags.value.bit_5(), "ROLLOVER", f)?;
        formatter.push(self.flags.value.bit_6(), "DISCONTINUITY", f)?;
        formatter.push(self.flags.value.bit_7(), "RESERVED(7)", f)?;
        FlagFormatter::end(f)
    }
}

pub(crate) struct AnalogFlagFormatter {
    flags: Flags,
}

impl AnalogFlagFormatter {
    pub(crate) fn new(value: u8) -> Self {
        Self {
            flags: Flags::new(value),
        }
    }
}

impl std::fmt::Display for AnalogFlagFormatter {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut formatter = FlagFormatter::new();
        FlagFormatter::begin(self.flags, f)?;
        formatter.format_binary_flags_0_to_4(self.flags, f)?;
        formatter.push(self.flags.value.bit_5(), "OVER_RANGE", f)?;
        formatter.push(self.flags.value.bit_6(), "REFERENCE_ERR", f)?;
        formatter.push(self.flags.value.bit_7(), "RESERVED(7)", f)?;
        FlagFormatter::end(f)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_16_bit_counter_keeps_the_low_16_bits() {
        assert_eq!(low_16_bits(0x0000_FFFF), 0xFFFF);
        assert_eq!(low_16_bits(0x0001_0000), 0);
        assert_eq!(low_16_bits(0x1234_5678), 0x5678);
    }

    fn analog(value: f64) -> AnalogInput {
        AnalogInput::new(value, Flags::ONLINE, Time::unsynchronized(0))
    }

    const OVER_RANGE: Flags = Flags::new(Flags::ONLINE.value | Flags::OVER_RANGE.value);

    #[test]
    fn i16_conversion_clamps_and_sets_over_range_on_both_sides() {
        let cases: &[(f64, Flags, i16)] = &[
            (0.0, Flags::ONLINE, 0),
            (-1.9, Flags::ONLINE, -1),
            (-32768.0, Flags::ONLINE, i16::MIN),
            (-32768.5, OVER_RANGE, i16::MIN),
            (-40000.0, OVER_RANGE, i16::MIN),
            (f64::NEG_INFINITY, OVER_RANGE, i16::MIN),
            (32767.0, Flags::ONLINE, i16::MAX),
            (32767.5, OVER_RANGE, i16::MAX),
            (40000.0, OVER_RANGE, i16::MAX),
            (f64::INFINITY, OVER_RANGE, i16::MAX),
            (f64::NAN, OVER_RANGE, 0),
        ];

        for &(value, flags, wire) in cases {
            assert_eq!(analog(value).to_i16(), (flags, wire), "value: {value}");
        }
    }

    #[test]
    fn i32_conversion_clamps_and_sets_over_range_on_both_sides() {
        let cases: &[(f64, Flags, i32)] = &[
            (0.0, Flags::ONLINE, 0),
            (40000.0, Flags::ONLINE, 40000),
            (-40000.0, Flags::ONLINE, -40000),
            (-2147483648.0, Flags::ONLINE, i32::MIN),
            (-2147483648.5, OVER_RANGE, i32::MIN),
            (-1e12, OVER_RANGE, i32::MIN),
            (f64::NEG_INFINITY, OVER_RANGE, i32::MIN),
            (2147483647.0, Flags::ONLINE, i32::MAX),
            (2147483647.5, OVER_RANGE, i32::MAX),
            (1e12, OVER_RANGE, i32::MAX),
            (f64::INFINITY, OVER_RANGE, i32::MAX),
            (f64::NAN, OVER_RANGE, 0),
        ];

        for &(value, flags, wire) in cases {
            assert_eq!(analog(value).to_i32(), (flags, wire), "value: {value}");
        }
    }

    #[test]
    fn f32_conversion_clamps_and_sets_over_range_on_both_sides() {
        // NaN has no single bit pattern, so compare bits only for numbers
        fn bits(value: f32) -> Option<u32> {
            (!value.is_nan()).then(|| value.to_bits())
        }

        let f32_max: f64 = f32::MAX.into();
        let f32_min: f64 = f32::MIN.into();

        let cases: &[(f64, Flags, f32)] = &[
            (0.0, Flags::ONLINE, 0.0),
            (0.5, Flags::ONLINE, 0.5),
            (-40000.0, Flags::ONLINE, -40000.0),
            (f32_min, Flags::ONLINE, f32::MIN),
            (f32_min.next_down(), OVER_RANGE, f32::MIN),
            (f64::MIN, OVER_RANGE, f32::MIN),
            (f64::NEG_INFINITY, Flags::ONLINE, f32::NEG_INFINITY),
            (f32_max, Flags::ONLINE, f32::MAX),
            (f32_max.next_up(), OVER_RANGE, f32::MAX),
            (f64::MAX, OVER_RANGE, f32::MAX),
            (f64::INFINITY, Flags::ONLINE, f32::INFINITY),
            (f64::NAN, Flags::ONLINE, f32::NAN),
        ];

        for &(value, flags, wire) in cases {
            let (actual_flags, actual_wire) = analog(value).to_f32();
            assert_eq!(
                (actual_flags, bits(actual_wire)),
                (flags, bits(wire)),
                "value: {value}"
            );
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
    fn formats_binary_flags() {
        assert_eq!(format!("{}", BinaryFlagFormatter::new(0)), "0x00 []");
        assert_eq!(
            format!("{}", BinaryFlagFormatter::new(0b1100_0001)),
            "0xC1 [ONLINE, RESERVED(6), STATE]"
        );
    }

    #[test]
    fn formats_double_flags() {
        assert_eq!(
            format!("{}", DoubleBitBinaryFlagFormatter::new(0)),
            "0x00 [state = Intermediate]"
        );
        assert_eq!(
            format!("{}", DoubleBitBinaryFlagFormatter::new(0b1100_0001)),
            "0xC1 [ONLINE, state = Indeterminate]"
        );
    }
}
