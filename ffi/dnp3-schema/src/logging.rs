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
        .doc(
            doc("Log level")
            .details("Used in {interface:Logger.on_message()} callback to identify the log level of a message.")
            .details("You can set the current log level with {class:Logging.SetLogLevel()}.")
        )?
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
            "logger",
            Type::Interface(log_callback_interface),
            "Logger that will receive each logged message",
        )?
        .return_type(ReturnType::void())?
        .doc(
            doc("Set the callback that will receive all the log messages")
            .details("There is only a single globally allocated logger. Calling this method a second time simply replaces the previous logger.")
            .details("If this method is never called, the default logger does nothing.")
        )?
        .build()?;

    let set_log_level_fn = lib
        .declare_native_function("logging_set_log_level")?
        .param("level", Type::Enum(log_level_enum), "Maximum log level")?
        .return_type(ReturnType::void())?
        .doc(
            doc("Set the current maximum log level")
                .details("All the log messages that are under that level will be discarded."),
        )?
        .build()?;

    let logging_class = lib.declare_class("Logging")?;
    let _logging_class = lib
        .define_class(&logging_class)?
        .static_method("SetHandler", &set_callback_fn)?
        .static_method("SetLogLevel", &set_log_level_fn)?
        .doc("Logging configuration class")?
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
