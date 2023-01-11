use oo_bindgen::model::*;

#[derive(PartialEq, Eq)]
enum AttributeType {
    List,
    String,
    UInt,
    Bool,
    Time,
    Float,
    OctetString,
    BitString,
    All,
}

struct KnownAttribute {
    variation: u8,
    name: &'static str,
    desc: &'static str,
    attr_type: AttributeType,
}

const fn string(variation: u8, name: &'static str, desc: &'static str) -> KnownAttribute {
    KnownAttribute::new(variation, name, desc, AttributeType::String)
}

const fn octet_string(variation: u8, name: &'static str, desc: &'static str) -> KnownAttribute {
    KnownAttribute::new(variation, name, desc, AttributeType::OctetString)
}

const fn time(variation: u8, name: &'static str, desc: &'static str) -> KnownAttribute {
    KnownAttribute::new(variation, name, desc, AttributeType::Time)
}

const fn float(variation: u8, name: &'static str, desc: &'static str) -> KnownAttribute {
    KnownAttribute::new(variation, name, desc, AttributeType::Float)
}

const fn uint(variation: u8, name: &'static str, desc: &'static str) -> KnownAttribute {
    KnownAttribute::new(variation, name, desc, AttributeType::UInt)
}

const fn bool(variation: u8, name: &'static str, desc: &'static str) -> KnownAttribute {
    KnownAttribute::new(variation, name, desc, AttributeType::Bool)
}

const fn list(variation: u8, name: &'static str, desc: &'static str) -> KnownAttribute {
    KnownAttribute::new(variation, name, desc, AttributeType::List)
}

const fn all(variation: u8, name: &'static str, desc: &'static str) -> KnownAttribute {
    KnownAttribute::new(variation, name, desc, AttributeType::All)
}

impl KnownAttribute {
    const fn new(
        variation: u8,
        name: &'static str,
        desc: &'static str,
        attr_type: AttributeType,
    ) -> Self {
        Self {
            variation,
            name,
            desc,
            attr_type,
        }
    }
}

const ATTRIBUTES: &[KnownAttribute] = &[
    string(196, "config_id", "Configuration id"),
    string(197, "config_version", "Configuration version"),
    time(198, "config_build_date", "Time and date that the outstation's current configuration was built defined"),
    time(199, "config_last_change_date","Time and date that the outstation's configuration was last modified"),
    octet_string(200, "config_digest", "Digest (aka fingerprint) of the configuration using a CRC, HASH, MAC, or public key signature"),
    string(201, "config_digest_algorithm", "Configuration digest algorithm"),
    string(202, "master_resource_id", "Master resource id (mRID)"),
    float(203, "device_location_altitude", "Altitude of the device"),
    float(204, "device_location_longitude", "Longitude of the device from reference meridian (-180.0 to 180.0 deg)"),
    float(205, "device_location_latitude", "Latitude of the device from the equator (90.0 to -90.0 deg)"),
    string(206, "user_assigned_secondary_operator_name", "User-assigned secondary operator name"),
    string(207, "user_assigned_primary_operator_name", "User-assigned primary operator name"),
    string(208,"user_assigned_system_name", "User-assigned system name"),
    uint(209, "secure_auth_version", "Secure authentication version"),
    uint(210, "num_security_stats_per_assoc", "Number of security statistics per association"),
    string(211, "user_specific_attributes", "Identification of user-specific attributes"),
    uint(212, "num_master_defined_data_set_proto", "Number of master defined data-set prototypes"),
    uint(213, "num_outstation_defined_data_set_proto", "Number of outstation defined data-set prototypes"),
    uint(214, "num_master_defined_data_sets", "Number of master defined data-sets"),
    uint(215, "num_outstation_defined_data_sets", "Number of outstation defined data-sets"),
    uint(216, "max_binary_output_per_request", "Maximum number of binary outputs per request"),
    uint(217, "local_timing_accuracy", "Local timing accuracy (microseconds)"),
    uint(218, "duration_of_time_accuracy", "Duration of time accuracy (seconds)"),
    bool(219, "supports_analog_output_events", "Supports analog output events"),
    uint(220, "max_analog_output_index", "Maximum analog output index"),
    uint(221, "num_analog_outputs", "Number of analog outputs"),
    bool(222, "supports_binary_output_events",  "Supports binary output events"),
    uint(223, "max_binary_output_index", "Maximum binary output index"),
    uint(224, "num_binary_outputs", "Number of binary outputs"),
    bool(225, "supports_frozen_counter_events", "Supports frozen counter events"),
    bool(226, "supports_frozen_counters", "Supports frozen counters"),
    bool(227, "supports_counter_events", "Supports counter events"),
    uint(228, "max_counter_index", "Maximum counter point index"),
    uint(229, "num_counter", "Number of counter points"),
    bool(230, "supports_frozen_analog_inputs", "Supports frozen analog input events"),
    bool(231, "supports_analog_input_events", "Supports analog input events"),
    uint(232, "max_analog_input_index", "Maximum analog input point index"),
    uint(233, "num_analog_input", "Number of analog input points"),
    bool(234, "supports_double_bit_binary_input_events", "Supports double-bit binary input events"),
    uint(235, "max_double_bit_binary_input_index", "Maximum double-bit binary input point index"),
    uint(236, "num_double_bit_binary_input", "Number of double-bit binary input points"),
    bool(237, "supports_binary_input_events", "Support binary input events"),
    uint(238, "max_binary_input_index", "Maximum binary input point index"),
    uint(239, "num_binary_input", "Number of binary input points"),
    uint(240, "max_tx_fragment_size", "Maximum transmit fragment size"),
    uint(241, "max_rx_fragment_size","Maximum receive fragment size"),
    string(242, "device_manufacturer_software_version", "Device manufacturer software version"),
    string(243,"device_manufacturer_hardware_version", "Device manufacturer hardware version"),
    string(244, "user_assigned_owner_name", "User-assigned owner name"),
    string(245, "user_assigned_location", "User assigned location name"),
    string(246, "user_assigned_id", "User assigned ID code/number"),
    string(247, "user_assigned_device_name", "User assigned device name"),
    string(248, "device_serial_number", "Device serial number"),
    string(249, "device_subset_and_conformance", "DNP3 subset and conformance"),
    string(250, "product_name_and_model", "Device manufacturer's product name and model"),
    string(252,"device_manufacturers_name", "Device manufacturer's name"),
    all(254, "all_attributes_request", "Non-specific all attributes request"),
    list(255, "list_of_variations", "List of attribute variations"),
];

pub(crate) struct DeviceAttrTypes {
    pub(crate) variation_list_attr: EnumHandle,
    pub(crate) string_attr: EnumHandle,
    pub(crate) uint_attr: EnumHandle,
    pub(crate) int_attr: EnumHandle,
    pub(crate) bool_attr: EnumHandle,
    pub(crate) time_attr: EnumHandle,
    pub(crate) octet_string_attr: EnumHandle,
    pub(crate) bit_string_attr: EnumHandle,
    pub(crate) float_attr: EnumHandle,
    pub(crate) attr_item_iter: AbstractIteratorHandle,
}

pub(crate) fn define(lib: &mut LibraryBuilder) -> BackTraced<DeviceAttrTypes> {
    define_attr_constants(lib)?;

    Ok(DeviceAttrTypes {
        variation_list_attr: define_variation_list_attr(lib)?,
        string_attr: define_string_attr(lib)?,
        uint_attr: define_uint_attr(lib)?,
        int_attr: define_int_attr(lib)?,
        bool_attr: define_bool_attr(lib)?,
        time_attr: define_time_attr(lib)?,
        octet_string_attr: define_octet_string_attr(lib)?,
        bit_string_attr: define_bit_string_attr(lib)?,
        float_attr: define_float_attr(lib)?,
        attr_item_iter: define_attr_item_iterator(lib)?,
    })
}

fn define_attr_constants(lib: &mut LibraryBuilder) -> BackTraced<()> {
    let mut builder = lib
        .define_constants("attribute_variations")?
        .doc("Device attribute variation constants")?;

    for attr in ATTRIBUTES {
        builder = builder.add(
            attr.name,
            ConstantValue::U8(attr.variation, Representation::Hex),
            attr.desc,
        )?;
    }

    builder.build()?;

    Ok(())
}

fn define_attr_item_struct(lib: &mut LibraryBuilder) -> BackTraced<UniversalStructHandle> {
    let attr_prop = lib.declare_universal_struct("attr_prop")?;
    let attr_prop = lib
        .define_universal_struct(attr_prop)?
        .doc(
            doc("Attribute properties returned in Group0Var255").details(
                "In 1815-2012 this only includes a field indicating if the property can be written",
            ),
        )?
        .add(
            "is_writable",
            Primitive::Bool,
            "Indicate if the property can be used in a WRITE operation",
        )?
        .end_fields()?
        .build()?;

    let attr_item = lib.declare_universal_struct("attr_item")?;
    let attr_item = lib
        .define_universal_struct(attr_item)?
        .doc("An attribute variation and properties pair returned in Group0Var255")?
        .add("variation", Primitive::U8, "Variation of the attribute")?
        .add("properties", attr_prop, "Properties of the attribute")?
        .end_fields()?
        .build()?;

    Ok(attr_item)
}

fn define_attr_item_iterator(lib: &mut LibraryBuilder) -> BackTraced<AbstractIteratorHandle> {
    let item = define_attr_item_struct(lib)?;

    let iter = lib.define_iterator("attr_item_iter", item)?;

    Ok(iter)
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

fn define_attr_enum(
    lib: &mut LibraryBuilder,
    typ: AttributeType,
    name: &str,
    doc: Doc<Unvalidated>,
) -> BackTraced<EnumHandle> {
    let mut builder = lib.define_enum(name)?.doc(doc)?.add_unknown()?;

    for attr in ATTRIBUTES.iter().filter(|x| x.attr_type == typ) {
        builder = builder.push(
            attr.name,
            &format!("Variation {} - {}", attr.variation, attr.desc),
        )?;
    }

    let value = builder.build()?;

    Ok(value)
}

fn define_variation_list_attr(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    define_attr_enum(
        lib,
        AttributeType::List,
        "variation_list_attr",
        doc("Enumeration of all the variation list attributes"),
    )
}

fn define_string_attr(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    define_attr_enum(
        lib,
        AttributeType::String,
        "string_attr",
        doc("Enumeration of all the default string attributes"),
    )
}

fn define_uint_attr(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    define_attr_enum(
        lib,
        AttributeType::UInt,
        "uint_attr",
        doc("Enumeration of all the default uint attributes"),
    )
}

fn define_int_attr(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let value = lib
        .define_enum("int_attr")?
        .doc(
            doc("Enumeration of all the default integer attributes")
                .details("In 1815-2012 all integer attributes are mapped to boolean values"),
        )?
        .add_unknown()?
        .build()?;

    Ok(value)
}

fn define_bool_attr(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    define_attr_enum(lib, AttributeType::Bool, "bool_attr", doc("Enumeration of all the known boolean attributes").details("Boolean attributes are actually just encoded as signed integer attributes where 1 == true"))
}

fn define_time_attr(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    define_attr_enum(
        lib,
        AttributeType::Time,
        "time_attr",
        doc("Enumeration of all the known DNP3 Time attributes"),
    )
}

fn define_octet_string_attr(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    define_attr_enum(
        lib,
        AttributeType::OctetString,
        "octet_string_attr",
        doc("Enumeration of all known octet-string attributes"),
    )
}

fn define_bit_string_attr(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    define_attr_enum(
        lib,
        AttributeType::BitString,
        "bit_string_attr",
        doc("Enumeration of all known bit-string attributes"),
    )
}

fn define_float_attr(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    define_attr_enum(
        lib,
        AttributeType::Float,
        "float_attr",
        doc("Enumeration of all known float attributes"),
    )
}
