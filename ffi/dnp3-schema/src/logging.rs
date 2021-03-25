use oo_bindgen::error_type::ErrorType;
use oo_bindgen::native_function::*;
use oo_bindgen::native_struct::{NativeStructHandle, StructElementType};
use oo_bindgen::*;

pub fn define(
    lib: &mut LibraryBuilder,
    error_type: ErrorType,
) -> Result<NativeStructHandle, BindingError> {
    let log_level_enum = lib
        .define_native_enum("LogLevel")?
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
        .define_native_enum("TimeFormat")?
        .push("None", "Don't format the timestamp as part of the message")?
        .push("Rfc3339", "Format the time using RFC 3339")?
        .push(
            "System",
            "Format the time in a human readable format e.g. 'Jun 25 14:27:12.955'",
        )?
        .doc("Describes if and how the time will be formatted in log messages")?
        .build()?;

    let log_output_format_enum = lib
        .define_native_enum("LogOutputFormat")?
        .push("Text", "A simple text-based format")?
        .push("Json", "Output formatted as JSON")?
        .doc("Describes how each log event is formatted")?
        .build()?;

    let logging_config_struct = lib.declare_native_struct("LoggingConfig")?;
    let logging_config_struct = lib
        .define_native_struct(&logging_config_struct)?
        .add(
            "level",
            StructElementType::Enum(log_level_enum.clone(), Some("Info".to_string())),
            "logging level",
        )?
        .add(
            "output_format",
            StructElementType::Enum(log_output_format_enum, Some("Text".to_string())),
            "output formatting options",
        )?
        .add(
            "time_format",
            StructElementType::Enum(time_format_enum, Some("System".to_string())),
            "optional time format",
        )?
        .add(
            "print_level",
            StructElementType::Bool(Some(true)),
            "optionally print the log level as part to the message string",
        )?
        .add(
            "print_module_info",
            StructElementType::Bool(Some(false)),
            "optionally print the underlying Rust module information to the message string",
        )?
        .doc("Logging configuration options")?
        .build()?;

    let log_callback_interface = lib
        .define_interface(
            "Logger",
            "Logging interface that receives the log messages and writes them somewhere.",
        )?
        .callback(
            "on_message",
            "Called when a log message was received and should be logged",
        )?
        .param("level", Type::Enum(log_level_enum), "Level of the message")?
        .param("message", Type::String, "Actual formatted message")?
        .return_type(ReturnType::void())?
        .build()?
        .destroy_callback("on_destroy")?
        .build()?;

    let configure_logging_fn = lib
        .declare_native_function("configure_logging")?
        .param(
            "config",
            Type::Struct(logging_config_struct),
            "Configuration options for logging"
        )?
        .param(
            "logger",
            Type::Interface(log_callback_interface),
            "Logger that will receive each logged message",
        )?
        .return_type(ReturnType::void())?
        .fails_with(error_type)?
        .doc(
            doc("Set the callback that will receive all the log messages")
            .details("There is only a single globally allocated logger. Calling this method a second time will result in a panic.")
            .details("If this method is never called, no logging will be performed.")
        )?
        .build()?;

    let _logging_class = lib
        .define_static_class("Logging")?
        .static_method("Configure", &configure_logging_fn)?
        .doc("Provides a static method for configuring logging")?
        .build()?;

    let app_decode_level_enum = lib
        .define_native_enum("AppDecodeLevel")?
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
        .define_native_enum("TransportDecodeLevel")?
        .push("Nothing", "Decode nothing")?
        .push("Header", " Decode the header")?
        .push("Payload", "Decode the header and the raw payload as hexadecimal")?
        .doc("Controls how transmitted and received transport segments are decoded at the INFO log level")?
        .build()?;

    let link_decode_level_enum = lib
        .define_native_enum("LinkDecodeLevel")?
        .push("Nothing", "Decode nothing")?
        .push("Header", " Decode the header")?
        .push(
            "Payload",
            "Decode the header and the raw payload as hexadecimal",
        )?
        .doc("Controls how transmitted and received link frames are decoded at the INFO log level")?
        .build()?;

    let phys_decode_level_enum = lib
        .define_native_enum("PhysDecodeLevel")?
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

    let decode_level_struct = lib.declare_native_struct("DecodeLevel")?;
    let decode_level_struct = lib.define_native_struct(&decode_level_struct)?
        .add("application", StructElementType::Enum(app_decode_level_enum, Some("Nothing".to_string())), "Controls application fragment decoding")?
        .add("transport", StructElementType::Enum(transport_decode_level_enum, Some("Nothing".to_string())), "Controls transport segment layer decoding")?
        .add("link", StructElementType::Enum(link_decode_level_enum, Some("Nothing".to_string())), "Controls link frame decoding")?
        .add("physical", StructElementType::Enum(phys_decode_level_enum, Some("Nothing".to_string())), "Controls the logging of physical layer read/write")?
        .doc("Controls the decoding of transmitted and received data at the application, transport, link, and physical layers")?
        .build()?;

    Ok(decode_level_struct)
}
