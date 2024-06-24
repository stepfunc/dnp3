use crate::app::attr::{AnyAttribute, Attribute};
use crate::app::measurement::*;
use crate::app::{MaybeAsync, QualifierCode, ResponseHeader, Variation};

/// Trait used to process measurement data received from an outstation
#[allow(unused_variables)]
pub trait ReadHandler: Send + Sync {
    /// Called as the first action before any of the type-specific handle methods are invoked
    ///
    /// `read_type` provides information about what triggered the call, e.g. response vs unsolicited
    /// `header` provides the full response header
    ///
    /// Note: The operation may or may not be async depending
    fn begin_fragment(&mut self, read_type: ReadType, header: ResponseHeader) -> MaybeAsync<()> {
        MaybeAsync::ready(())
    }

    /// Called as the last action after all the type-specific handle methods have been invoked
    ///
    /// `read_type` provides information about what triggered the call, e.g. response vs unsolicited
    /// `header` provides the full response header
    ///
    /// Note: The operation may or may not be async depending. A typical use case for using async
    /// here would be to publish a message to an async MPSC.
    fn end_fragment(&mut self, read_type: ReadType, header: ResponseHeader) -> MaybeAsync<()> {
        MaybeAsync::ready(())
    }

    /// Process an object header of `BinaryInput` values
    fn handle_binary_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryInput, u16)>,
    ) {
    }

    /// Process an object header of `DoubleBitBinaryInput` values
    fn handle_double_bit_binary_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (DoubleBitBinaryInput, u16)>,
    ) {
    }

    /// Process an object header of `BinaryOutputStatus` values
    fn handle_binary_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryOutputStatus, u16)>,
    ) {
    }

    /// Process an object header of `Counter` values
    fn handle_counter(&mut self, info: HeaderInfo, iter: &mut dyn Iterator<Item = (Counter, u16)>) {
    }

    /// Process an object header of `FrozenCounter` values
    fn handle_frozen_counter(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (FrozenCounter, u16)>,
    ) {
    }

    /// Process an object header of `AnalogInput` values
    fn handle_analog_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogInput, u16)>,
    ) {
    }

    /// Process an object header of `FrozenAnalogInput` values
    fn handle_frozen_analog_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (FrozenAnalogInput, u16)>,
    ) {
    }

    /// Process an object header of `AnalogInputDeadBand` values
    fn handle_analog_input_dead_band(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogInputDeadBand, u16)>,
    ) {
    }

    /// Process an object header of `AnalogOutputStatus` values
    fn handle_analog_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogOutputStatus, u16)>,
    ) {
    }

    /// Process an object header of `AnalogOutputCommandEvent` values
    fn handle_analog_output_command_event(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogOutputCommandEvent, u16)>,
    ) {
    }

    /// Process an object header of `BinaryOutputCommandEvent` values
    fn handle_binary_output_command_event(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryOutputCommandEvent, u16)>,
    ) {
    }

    /// Process an object header of `UnsignedInteger` values
    fn handle_unsigned_integer(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (UnsignedInteger, u16)>,
    ) {
    }

    /// Process an object header of octet string values
    fn handle_octet_string<'a>(
        &mut self,
        info: HeaderInfo,
        iter: &'a mut dyn Iterator<Item = (&'a [u8], u16)>,
    ) {
    }

    /// Process a device attribute
    fn handle_device_attribute(&mut self, info: HeaderInfo, attr: AnyAttribute) {}
}

pub(crate) fn handle_attribute(
    var: Variation,
    qualifier: QualifierCode,
    attr: &Option<Attribute>,
    handler: &mut dyn ReadHandler,
) {
    if let Some(attr) = attr {
        match AnyAttribute::try_from(attr) {
            Ok(attr) => {
                handler
                    .handle_device_attribute(HeaderInfo::new(var, qualifier, false, false), attr);
            }
            Err(err) => {
                tracing::warn!(
                    "Expected attribute type {:?} but received {:?}",
                    err.expected,
                    err.actual
                );
            }
        }
    }
}

/// Information about the object header and specific variation
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HeaderInfo {
    /// underlying variation in the response
    pub variation: Variation,
    /// qualifier code used in the response
    pub qualifier: QualifierCode,
    /// true if the received variation is an event type, false otherwise
    pub is_event: bool,
    /// true if a flags byte is present on the underlying variation, false otherwise
    pub has_flags: bool,
}

impl HeaderInfo {
    pub(crate) fn new(
        variation: Variation,
        qualifier: QualifierCode,
        is_event: bool,
        has_flags: bool,
    ) -> Self {
        Self {
            variation,
            qualifier,
            is_event,
            has_flags,
        }
    }
}

/// Describes the source of a read event
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ReadType {
    /// Startup integrity poll
    StartupIntegrity,
    /// Unsolicited message
    Unsolicited,
    /// Single poll requested by the user
    SinglePoll,
    /// Periodic poll configured by the user
    PeriodicPoll,
}
