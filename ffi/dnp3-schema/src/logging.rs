use oo_bindgen::enum_type::EnumHandle;
use oo_bindgen::error_type::ErrorType;
use oo_bindgen::structs::{
    ConstructorType, FieldName, FunctionArgStructHandle, ToDefaultVariant, UniversalStructHandle,
};
use oo_bindgen::types::{BasicType, StringType};
use oo_bindgen::*;

fn define_log_level_enum(lib: &mut LibraryBuilder) -> BindResult<EnumHandle> {
    lib
        .define_enum("LogLevel")
        .push("Error", "Error log level")?
        .push("Warn", "Warning log level")?
        .push("Info", "Information log level")?
        .push("Debug", "Debugging log level")?
        .push("Trace", "Trace log level")?
        .doc(
            doc("Log level")
                .details("Used in {interface:Logger.on_message()} callback to identify the log level of a message.")
        )?
        .build()
}

fn define_time_format_enum(lib: &mut LibraryBuilder) -> BindResult<EnumHandle> {
    lib.define_enum("TimeFormat")
        .push("None", "Don't format the timestamp as part of the message")?
        .push("Rfc3339", "Format the time using RFC 3339")?
        .push(
            "System",
            "Format the time in a human readable format e.g. 'Jun 25 14:27:12.955'",
        )?
        .doc("Describes if and how the time will be formatted in log messages")?
        .build()
}

fn define_log_output_format_enum(lib: &mut LibraryBuilder) -> BindResult<EnumHandle> {
    lib.define_enum("LogOutputFormat")
        .push("Text", "A simple text-based format")?
        .push("Json", "Output formatted as JSON")?
        .doc("Describes how each log event is formatted")?
        .build()
}

fn define_logging_config_struct(
    lib: &mut LibraryBuilder,
    log_level_enum: EnumHandle,
) -> BindResult<FunctionArgStructHandle> {
    let logging_config_struct = lib.declare_function_arg_struct("LoggingConfig")?;

    let log_output_format_enum = define_log_output_format_enum(lib)?;
    let time_format_enum = define_time_format_enum(lib)?;

    let level = FieldName::new("level");
    let output_format = FieldName::new("output_format");
    let time_format = FieldName::new("time_format");
    let print_level = FieldName::new("print_level");
    let print_module_info = FieldName::new("print_module_info");

    lib.define_function_argument_struct(logging_config_struct)?
        .add(&level, log_level_enum, "logging level")?
        .add(
            &output_format,
            log_output_format_enum,
            "output formatting options",
        )?
        .add(&time_format, time_format_enum, "optional time format")?
        .add(
            &print_level,
            BasicType::Bool,
            "optionally print the log level as part to the message string",
        )?
        .add(
            &print_module_info,
            BasicType::Bool,
            "optionally print the underlying Rust module information to the message string",
        )?
        .doc("Logging configuration options")?
        .end_fields()?
        .begin_constructor(
            "init",
            ConstructorType::Normal,
            "Initialize the configuration to default values",
        )?
        .default(&level, "Info".default_variant())?
        .default(&output_format, "Text".default_variant())?
        .default(&time_format, "System".default_variant())?
        .default(&print_level, true)?
        .default(&print_module_info, false)?
        .end_constructor()?
        .build()
}

const NOTHING: &str = "Nothing";

pub fn define(
    lib: &mut LibraryBuilder,
    error_type: ErrorType,
) -> Result<UniversalStructHandle, BindingError> {
    let log_level_enum = define_log_level_enum(lib)?;

    let logging_config_struct = define_logging_config_struct(lib, log_level_enum.clone())?;

    let log_callback_interface = lib
        .define_asynchronous_interface(
            "Logger",
            "Logging interface that receives the log messages and writes them somewhere.",
        )
        .begin_callback(
            "on_message",
            "Called when a log message was received and should be logged",
        )?
        .param("level", log_level_enum, "Level of the message")?
        .param("message", StringType, "Actual formatted message")?
        .returns_nothing()?
        .end_callback()?
        .build()?;

    let configure_logging_fn = lib
        .define_function("configure_logging")
        .param(
            "config",
           logging_config_struct,
            "Configuration options for logging"
        )?
        .param(
            "logger",
           log_callback_interface,
            "Logger that will receive each logged message",
        )?
        .returns_nothing()?
        .fails_with(error_type)?
        .doc(
            doc("Set the callback that will receive all the log messages")
            .details("There is only a single globally allocated logger. Calling this method a second time will return an error.")
            .details("If this method is never called, no logging will be performed.")
        )?
        .build()?;

    let _logging_class = lib
        .define_static_class("Logging")
        .static_method("Configure", &configure_logging_fn)?
        .doc("Provides a static method for configuring logging")?
        .build()?;

    let app_decode_level_enum = lib
        .define_enum("AppDecodeLevel")
        .push(NOTHING, "Decode nothing")?
        .push("Header", " Decode the header-only")?
        .push("ObjectHeaders", "Decode the header and the object headers")?
        .push(
            "ObjectValues",
            "Decode the header, the object headers, and the object values",
        )?
        .doc("Controls how transmitted and received application-layer fragments are decoded at the INFO log level")?
        .build()?;

    let transport_decode_level_enum = lib
        .define_enum("TransportDecodeLevel")
        .push(NOTHING, "Decode nothing")?
        .push("Header", " Decode the header")?
        .push("Payload", "Decode the header and the raw payload as hexadecimal")?
        .doc("Controls how transmitted and received transport segments are decoded at the INFO log level")?
        .build()?;

    let link_decode_level_enum = lib
        .define_enum("LinkDecodeLevel")
        .push(NOTHING, "Decode nothing")?
        .push("Header", " Decode the header")?
        .push(
            "Payload",
            "Decode the header and the raw payload as hexadecimal",
        )?
        .doc("Controls how transmitted and received link frames are decoded at the INFO log level")?
        .build()?;

    let phys_decode_level_enum = lib
        .define_enum("PhysDecodeLevel")
        .push(NOTHING, "Log nothing")?
        .push(
            "Length",
            "Log only the length of data that is sent and received",
        )?
        .push(
            "Data",
            "Log the length and the actual data that is sent and received",
        )?
        .doc("Controls how data transmitted at the physical layer (TCP, serial, etc) is logged")?
        .build()?;

    let application_field = FieldName::new("application");
    let transport_field = FieldName::new("transport");
    let link_field = FieldName::new("link");
    let physical_field = FieldName::new("physical");

    let decode_level_struct = lib.declare_universal_struct("DecodeLevel")?;
    let decode_level_struct = lib.define_universal_struct(decode_level_struct)?
        .add(&application_field, app_decode_level_enum, "Controls application fragment decoding")?
        .add(&transport_field, transport_decode_level_enum, "Controls transport segment layer decoding")?
        .add(&link_field, link_decode_level_enum, "Controls link frame decoding")?
        .add(&physical_field, phys_decode_level_enum, "Controls the logging of physical layer read/write")?
        .doc("Controls the decoding of transmitted and received data at the application, transport, link, and physical layers")?
        .end_fields()?
        .begin_constructor("init", ConstructorType::Normal, "Initialize log levels to defaults")?
        .default_variant(&application_field, NOTHING)?
        .default_variant(&transport_field, NOTHING)?
        .default_variant(&link_field, NOTHING)?
        .default_variant(&physical_field, NOTHING)?
        .end_constructor()?
        .build()?;

    Ok(decode_level_struct)
}
