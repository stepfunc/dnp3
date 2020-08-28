use oo_bindgen::native_enum::*;
use oo_bindgen::native_function::*;
use oo_bindgen::*;

pub fn define(lib: &mut LibraryBuilder) -> Result<NativeEnumHandle, BindingError> {
    let log_level_enum = lib
        .define_native_enum("LogLevel")?
        .push("Error", "Error log level")?
        .push("Warn", "Warning log level")?
        .push("Info", "Information log level")?
        .push("Debug", "Debugging log level")?
        .push("Trace", "Trace log level")?
        .doc("Log level")?
        .build()?;

    let log_callback_interface = lib
        .define_interface("Logger", "Logging interface")?
        .callback("on_message", "Called when a message should be logged")?
        .param(
            "level",
            Type::Enum(log_level_enum.clone()),
            "Level of the message",
        )?
        .param("message", Type::String, "Actual formatted message")?
        .return_type(ReturnType::void())?
        .build()?
        .destroy_callback("on_destroy")?
        .build()?;

    let set_callback_fn = lib
        .declare_native_function("logging_set_callback")?
        .param(
            "handler",
            Type::Interface(log_callback_interface),
            "Handler that will receive each logged message",
        )?
        .return_type(ReturnType::void())?
        .doc("Set the callback that will receive all the log messages")?
        .build()?;

    let set_log_level_fn = lib
        .declare_native_function("logging_set_log_level")?
        .param("level", Type::Enum(log_level_enum), "Maximum log level")?
        .return_type(ReturnType::void())?
        .doc("Set the current log level")?
        .build()?;

    let logging_class = lib.declare_class("Logging")?;
    let _logging_class = lib
        .define_class(&logging_class)?
        .static_method("SetHandler", &set_callback_fn)?
        .static_method("SetLogLevel", &set_log_level_fn)?
        .doc("Helper functions for logging")?
        .build()?;

    let decode_log_level_enum = lib
        .define_native_enum("DecodeLogLevel")?
        .push("Nothing", "Decode nothing")?
        .push("Header", "Decode only the application layer header")?
        .push("ObjectHeaders", "Decode down to the object header values")?
        .push("ObjectValues", "Decode down to the object values")?
        .doc("Master decoding log level")?
        .build()?;

    Ok(decode_log_level_enum)
}
