use oo_bindgen::model::{doc, BackTraced, BindResult, EnumBuilder, EnumHandle, LibraryBuilder};

pub(crate) struct DeviceAttrTypes {
    pub(crate) string_attr: EnumHandle,
    pub(crate) uint_attr: EnumHandle,
    pub(crate) bool_attr: EnumHandle,
    pub(crate) time_attr: EnumHandle,
    pub(crate) octet_string_attr: EnumHandle,
    pub(crate) float_attr: EnumHandle,
}

pub(crate) fn define(lib: &mut LibraryBuilder) -> BackTraced<DeviceAttrTypes> {
    Ok(DeviceAttrTypes {
        string_attr: define_string_attr(lib)?,
        uint_attr: define_uint_attr(lib)?,
        bool_attr: define_bool_attr(lib)?,
        time_attr: define_time_attr(lib)?,
        octet_string_attr: define_octet_string_attr(lib)?,
        float_attr: define_float_attr(lib)?,
    })
}

trait UnknownCase {
    fn add_unknown(self) -> BindResult<Self>
    where
        Self: Sized;
}

impl<'a> UnknownCase for EnumBuilder<'a> {
    fn add_unknown(self) -> BindResult<Self> {
        self.push(
            "unknown",
            "The attribute variation is not defined or is not part of the default set",
        )
    }
}

fn define_string_attr(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let value = lib
        .define_enum("string_attr")?
        .doc("Enumeration of all the default string attributes")?
        .push("config_id", "Variation 196 - Configuration id")?
        .push("config_version", "Variation 197 - Configuration version")?
        .push(
            "config_digest_algorithm",
            "Variation 201 - Configuration digest algorithm",
        )?
        .push(
            "master_resource_id",
            "Variation 202 - Master resource id (mRID)",
        )?
        .push(
            "user_assigned_secondary_operator_name",
            "Variation 206 - User-assigned secondary operator name",
        )?
        .push(
            "user_assigned_primary_operator_name",
            "Variation 207 - User-assigned primary operator name",
        )?
        .push(
            "user_assigned_system_name",
            "Variation 208 - User-assigned system name",
        )?
        .push(
            "user_specific_attributes",
            "Variation 211 - Identification of user-specific attributes",
        )?
        .push(
            "device_manufacturer_software_version",
            "Variation 242 - Device manufacturer software version",
        )?
        .push(
            "device_manufacturer_hardware_version",
            "Variation 243 - Device manufacturer hardware version",
        )?
        .push(
            "user_assigned_owner_name",
            "Variation 244 - User-assigned owner name",
        )?
        .push(
            "user_assigned_location",
            "Variation 245 - User assigned location/name",
        )?
        .push(
            "user_assigned_id",
            "Variation 246 - User assigned ID code/number",
        )?
        .push(
            "user_assigned_device_name",
            "Variation 247 - User assigned device name",
        )?
        .push(
            "device_serial_number",
            "Variation 248 - Device serial number",
        )?
        .push(
            "device_subset_and_conformance",
            "Variation 249 - DNP3 subset and conformance",
        )?
        .push(
            "product_name_and_model",
            "Variation 250 - Device manufacturer's product name and model",
        )?
        .push(
            "device_manufacturers_name",
            "Variation 252 - Device manufacturer's name",
        )?
        .add_unknown()?
        .build()?;

    Ok(value)
}

fn define_uint_attr(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let value = lib
        .define_enum("uint_attr")?
        .doc("Enumeration of all the default string attributes")?
        .push(
            "secure_auth_version",
            "Variation 209 - Secure authentication version",
        )?
        .push(
            "mum_security_stats_per_assoc",
            "Variation 210 - Number of security statistics per association",
        )?
        .push(
            "num_master_defined_data_set_proto",
            "Variation 212 - Number of master defined data-set prototypes",
        )?
        .push(
            "num_outstation_defined_data_set_proto",
            "Number of outstation defined data-set prototypes",
        )?
        .push(
            "num_master_defined_data_sets",
            "Variation 214 - Number of master defined data-sets",
        )?
        .push(
            "num_outstation_defined_data_sets",
            "Variation 215 - Number of outstation defined data-sets",
        )?
        .push(
            "max_binary_output_per_request",
            "Variation 216 - Maximum number of binary outputs per request",
        )?
        .push(
            "local_timing_accuracy",
            "Variation 217 - Local timing accuracy (microseconds)",
        )?
        .push(
            "duration_of_time_accuracy",
            "Variation 218 - Duration of time accuracy (seconds)",
        )?
        .push(
            "max_analog_output_index",
            "Variation 220 - Maximum analog output index",
        )?
        .push(
            "num_analog_outputs",
            "Variation 221 - Number of analog outputs",
        )?
        .push(
            "max_binary_output_index",
            "Variation 223 - Maximum binary output index",
        )?
        .push(
            "num_binary_outputs",
            "Variation 224 - Number of binary outputs",
        )?
        .push(
            "max_counter_index",
            "Variation 228 - Maximum counter point index",
        )?
        .push("num_counter", "Variation 229 - Number of counter points")?
        .push(
            "max_analog_input_index",
            "Variation 232 - Maximum analog input point index",
        )?
        .push(
            "num_analog_input",
            "Variation 233 - Number of analog input points",
        )?
        .push(
            "max_double_bit_binary_input_index",
            "Variation 235 - Maximum double-bit binary input point index",
        )?
        .push(
            "num_double_bit_binary_input",
            "Variation 236 - Number of double-bit binary input points",
        )?
        .push(
            "max_binary_input_index",
            "Variation 238 - Maximum binary input point index",
        )?
        .push(
            "num_binary_input",
            "Variation 239 - Number of binary input points",
        )?
        .push(
            "max_tx_fragment_size",
            "Variation 240 - Maximum transmit fragment size",
        )?
        .push(
            "max_rx_fragment_size",
            "Variation 241 - Maximum receive fragment size",
        )?
        .add_unknown()?
        .build()?;

    Ok(value)
}

fn define_bool_attr(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let value = lib.define_enum("bool_attr")?
        .doc(doc("Enumeration of all the known boolean attributes").details("Boolean attributes are actually just encoded as signed integer attributes where 1 == true"))?
        .push("supports_analog_output_events", "Variation 219 - Supports analog output events")?
        .push("supports_binary_output_events", "Variation 222 - Supports binary output events")?
        .push("supports_frozen_counter_events", "Variation 225 - Supports frozen counter events")?
        .push("supports_frozen_counters", "Variation 226 - Supports frozen counters")?
        .push("supports_counter_events", "Variation 227 - Supports counter events")?
        .push("supports_frozen_analog_inputs", "Variation 230 - Support frozen analog input events")?
        .push("supports_analog_input_events","Variation 231 - Support analog input events")?
        .push("supports_double_bit_binary_input_events", "Variation 234 - Support double-bit binary input events")?
        .push("supports_binary_input_events","Variation 237 - Support binary input events")?
        .add_unknown()?
        .build()?;

    Ok(value)
}

fn define_time_attr(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let value = lib.define_enum("time_attr")?
        .doc("Enumeration of all the known DNP3 Time attributes")?
        .push("config_build_date", "Variation 198 - Time and date that the outstation's current configuration was built defined")?
        .push("config_last_change_date", "Variation 199 - Time and date that the outstation's configuration was last modified")?
        .add_unknown()?
        .build()?;

    Ok(value)
}

fn define_octet_string_attr(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let value = lib.define_enum("octet_string_attr")?
        .doc("Enumeration of all known octet-string attributes")?
        .push("config_digest", "Variation 201 - Digest (aka fingerprint) of the configuration using a CRC, HASH, MAC, or public key signature")?
        .add_unknown()?
        .build()?;

    Ok(value)
}

fn define_float_attr(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let value = lib
        .define_enum("float_attr")?
        .doc("Enumeration of all known float attributes")?
        .push(
            "device_location_altitude",
            "Variation 203 - Altitude of the device",
        )?
        .push(
            "device_location_longitude",
            "Variation 204 - Longitude of the device from reference meridian (-180.0 to 180.0 deg)",
        )?
        .push(
            "device_location_latitude",
            "Variation 205 - Latitude of the device from the equator (90.0 to -90.0 deg)",
        )?
        .add_unknown()?
        .build()?;

    Ok(value)
}
