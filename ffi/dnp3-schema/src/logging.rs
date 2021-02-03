use oo_bindgen::native_enum::*;
use oo_bindgen::native_function::*;
use oo_bindgen::native_struct::StructElementType;
use oo_bindgen::*;

pub fn define(lib: &mut LibraryBuilder) -> Result<NativeEnumHandle, BindingError> {
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
        .push("RFC3339", "Format the time using RFC 3339")?
        .push(
            "System",
            "Format the time in a human readable format e.g. 'Jun 25 14:27:12.955'",
        )?
        .doc("Describes if and how the time will be formatted in log messages")?
        .build()?;

    let log_output_format_enum = lib
        .define_native_enum("LogOutputFormat")?
        .push("Text", "A simple text-based format")?
        .push("JSON", "Output formatted as JSON")?
        .doc("Describes how each log event is formatted")?
        .build()?;

    let logging_config_struct = lib.declare_native_struct("LoggingConfiguration")?;
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
        .doc(
            doc("Set the callback that will receive all the log messages")
            .details("There is only a single globally allocated logger. Calling this method a second time will result in a panic.")
            .details("If this method is never called, no logging will be performed.")
        )?
        .build()?;

    let logging_class = lib.declare_class("Logging")?;
    let _logging_class = lib
        .define_class(&logging_class)?
        .static_method("Configure", &configure_logging_fn)?
        .doc("Provides a static method for configuring logging")?
        .build()?;

    let decode_log_level_enum = lib
        .define_native_enum("DecodeLogLevel")?
        .push("Nothing", "Decode nothing")?
        .push("Header", "Decode only the application layer header")?
        .push("ObjectHeaders", "Decode down to the object header values")?
        .push("ObjectValues", "Decode down to the object values")?
        .doc(
            doc("Master decoding log level")
                .details("Determines how deep the master should decode and log the responses.")
                .details("Use {class:Master.SetDecodeLogLevel()} to set it."),
        )?
        .build()?;

    Ok(decode_log_level_enum)
}
