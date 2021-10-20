use oo_bindgen::error_type::ErrorType;
use oo_bindgen::structs::UniversalStructHandle;
use oo_bindgen::types::{BasicType, StringType};
use oo_bindgen::*;

pub fn define(
    lib: &mut LibraryBuilder,
    error_type: ErrorType,
) -> Result<UniversalStructHandle, BindingError> {
    let log_level_enum = lib
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
        .build()?;

    let time_format_enum = lib
        .define_enum("TimeFormat")
        .push("None", "Don't format the timestamp as part of the message")?
        .push("Rfc3339", "Format the time using RFC 3339")?
        .push(
            "System",
            "Format the time in a human readable format e.g. 'Jun 25 14:27:12.955'",
        )?
        .doc("Describes if and how the time will be formatted in log messages")?
        .build()?;

    let log_output_format_enum = lib
        .define_enum("LogOutputFormat")
        .push("Text", "A simple text-based format")?
        .push("Json", "Output formatted as JSON")?
        .doc("Describes how each log event is formatted")?
        .build()?;

    let logging_config_struct = lib.declare_struct("LoggingConfig")?;
    let logging_config_struct = lib
        .define_function_argument_struct(&logging_config_struct)?
        .add("level", log_level_enum.clone(), "logging level")?
        .add(
            "output_format",
            log_output_format_enum,
            "output formatting options",
        )?
        .add("time_format", time_format_enum, "optional time format")?
        .add(
            "print_level",
            BasicType::Bool,
            "optionally print the log level as part to the message string",
        )?
        .add(
            "print_module_info",
            BasicType::Bool,
            "optionally print the underlying Rust module information to the message string",
        )?
        .doc("Logging configuration options")?
        .end_fields()?
        // TODO - constructor
        .build()?;

    let log_callback_interface = lib
        .define_interface(
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
        .push("Nothing", "Decode nothing")?
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
        .push("Nothing", "Decode nothing")?
        .push("Header", " Decode the header")?
        .push("Payload", "Decode the header and the raw payload as hexadecimal")?
        .doc("Controls how transmitted and received transport segments are decoded at the INFO log level")?
        .build()?;

    let link_decode_level_enum = lib
        .define_enum("LinkDecodeLevel")
        .push("Nothing", "Decode nothing")?
        .push("Header", " Decode the header")?
        .push(
            "Payload",
            "Decode the header and the raw payload as hexadecimal",
        )?
        .doc("Controls how transmitted and received link frames are decoded at the INFO log level")?
        .build()?;

    let phys_decode_level_enum = lib
        .define_enum("PhysDecodeLevel")
        .push("Nothing", "Log nothing")?
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

    let decode_level_struct = lib.declare_struct("DecodeLevel")?;
    let decode_level_struct = lib.define_universal_struct(&decode_level_struct)?
        .add("application", app_decode_level_enum, "Controls application fragment decoding")?
        .add("transport", transport_decode_level_enum, "Controls transport segment layer decoding")?
        .add("link", link_decode_level_enum, "Controls link frame decoding")?
        .add("physical", phys_decode_level_enum, "Controls the logging of physical layer read/write")?
        .doc("Controls the decoding of transmitted and received data at the application, transport, link, and physical layers")?
        .end_fields()?
        // TODO - constructor
        .build()?;

    Ok(decode_level_struct)
}
