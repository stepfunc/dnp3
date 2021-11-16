use oo_bindgen::model::*;

fn define_log_level_enum(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let definition = lib
        .define_enum("log_level")?
        .push("error", "Error log level")?
        .push("warn", "Warning log level")?
        .push("info", "Information log level")?
        .push("debug", "Debugging log level")?
        .push("trace", "Trace log level")?
        .doc(
            doc("Log level")
                .details("Used in {interface:logger.on_message()} callback to identify the log level of a message.")
        )?
        .build()?;

    Ok(definition)
}

fn define_time_format_enum(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let definition = lib
        .define_enum("time_format")?
        .push("none", "Don't format the timestamp as part of the message")?
        .push("rfc_3339", "Format the time using RFC 3339")?
        .push(
            "system",
            "Format the time in a human readable format e.g. 'Jun 25 14:27:12.955'",
        )?
        .doc("Describes if and how the time will be formatted in log messages")?
        .build()?;

    Ok(definition)
}

fn define_log_output_format_enum(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let definition = lib
        .define_enum("log_output_format")?
        .push("text", "A simple text-based format")?
        .push("json", "Output formatted as JSON")?
        .doc("Describes how each log event is formatted")?
        .build()?;

    Ok(definition)
}

fn define_logging_config_struct(
    lib: &mut LibraryBuilder,
    log_level_enum: EnumHandle,
) -> BackTraced<FunctionArgStructHandle> {
    let logging_config_struct = lib.declare_function_arg_struct("logging_config")?;

    let log_output_format_enum = define_log_output_format_enum(lib)?;
    let time_format_enum = define_time_format_enum(lib)?;

    let level = Name::create("level")?;
    let output_format = Name::create("output_format")?;
    let time_format = Name::create("time_format")?;
    let print_level = Name::create("print_level")?;
    let print_module_info = Name::create("print_module_info")?;

    let logging_config_struct = lib
        .define_function_argument_struct(logging_config_struct)?
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
        .begin_initializer(
            "init",
            InitializerType::Normal,
            "Initialize the configuration to default values",
        )?
        .default(&level, "info".default_variant())?
        .default(&output_format, "text".default_variant())?
        .default(&time_format, "system".default_variant())?
        .default(&print_level, true)?
        .default(&print_module_info, false)?
        .end_initializer()?
        .build()?;

    Ok(logging_config_struct)
}

const NOTHING: &str = "nothing";

pub fn define(
    lib: &mut LibraryBuilder,
    error_type: ErrorType<Unvalidated>,
) -> BackTraced<UniversalStructHandle> {
    let log_level_enum = define_log_level_enum(lib)?;

    let logging_config_struct = define_logging_config_struct(lib, log_level_enum.clone())?;

    let log_callback_interface = lib
        .define_interface(
            "logger",
            "Logging interface that receives the log messages and writes them somewhere.",
        )?
        .begin_callback(
            "on_message",
            "Called when a log message was received and should be logged",
        )?
        .param("level", log_level_enum, "Level of the message")?
        .param("message", StringType, "Actual formatted message")?
        .end_callback()?
        .build_async()?;

    let configure_logging_fn = lib
        .define_function("configure_logging")?
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
        .fails_with(error_type)?
        .doc(
            doc("Set the callback that will receive all the log messages")
            .details("There is only a single globally allocated logger. Calling this method a second time will return an error.")
            .details("If this method is never called, no logging will be performed.")
        )?
        .build_static("configure")?;

    let _logging_class = lib
        .define_static_class("logging")?
        .static_method(configure_logging_fn)?
        .doc("Provides a static method for configuring logging")?
        .build()?;

    let app_decode_level_enum = lib
        .define_enum("app_decode_level")?
        .push(NOTHING, "Decode nothing")?
        .push("header", " Decode the header-only")?
        .push("object_headers", "Decode the header and the object headers")?
        .push(
            "object_values",
            "Decode the header, the object headers, and the object values",
        )?
        .doc("Controls how transmitted and received application-layer fragments are decoded at the INFO log level")?
        .build()?;

    let transport_decode_level_enum = lib
        .define_enum("transport_decode_level")?
        .push(NOTHING, "Decode nothing")?
        .push("header", " Decode the header")?
        .push("payload", "Decode the header and the raw payload as hexadecimal")?
        .doc("Controls how transmitted and received transport segments are decoded at the INFO log level")?
        .build()?;

    let link_decode_level_enum = lib
        .define_enum("link_decode_level")?
        .push(NOTHING, "Decode nothing")?
        .push("header", " Decode the header")?
        .push(
            "payload",
            "Decode the header and the raw payload as hexadecimal",
        )?
        .doc("Controls how transmitted and received link frames are decoded at the INFO log level")?
        .build()?;

    let phys_decode_level_enum = lib
        .define_enum("phys_decode_level")?
        .push(NOTHING, "Log nothing")?
        .push(
            "length",
            "Log only the length of data that is sent and received",
        )?
        .push(
            "data",
            "Log the length and the actual data that is sent and received",
        )?
        .doc("Controls how data transmitted at the physical layer (TCP, serial, etc) is logged")?
        .build()?;

    let application_field = Name::create("application")?;
    let transport_field = Name::create("transport")?;
    let link_field = Name::create("link")?;
    let physical_field = Name::create("physical")?;

    let decode_level_struct = lib.declare_universal_struct("decode_level")?;
    let decode_level_struct = lib.define_universal_struct(decode_level_struct)?
        .add(&application_field, app_decode_level_enum, "Controls application fragment decoding")?
        .add(&transport_field, transport_decode_level_enum, "Controls transport segment layer decoding")?
        .add(&link_field, link_decode_level_enum, "Controls link frame decoding")?
        .add(&physical_field, phys_decode_level_enum, "Controls the logging of physical layer read/write")?
        .doc("Controls the decoding of transmitted and received data at the application, transport, link, and physical layers")?
        .end_fields()?
        .begin_initializer("init", InitializerType::Normal, "Initialize log levels to defaults")?
        .default_variant(&application_field, NOTHING)?
        .default_variant(&transport_field, NOTHING)?
        .default_variant(&link_field, NOTHING)?
        .default_variant(&physical_field, NOTHING)?
        .end_initializer()?
        .build()?;

    Ok(decode_level_struct)
}
