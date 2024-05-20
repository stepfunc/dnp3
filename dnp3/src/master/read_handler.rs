use crate::app::attr::AnyAttribute;
use crate::app::measurement::*;
use crate::app::{MaybeAsync, ResponseHeader};
use crate::master::{HeaderInfo, ReadType};

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
