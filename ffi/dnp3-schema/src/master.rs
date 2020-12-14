use oo_bindgen::callback::InterfaceHandle;
use oo_bindgen::class::ClassDeclarationHandle;
use oo_bindgen::native_enum::*;
use oo_bindgen::native_function::*;
use oo_bindgen::native_struct::*;
use oo_bindgen::*;

pub fn define(
    lib: &mut LibraryBuilder,
    master_class: ClassDeclarationHandle,
    read_handler: InterfaceHandle,
    decode_log_level_enum: NativeEnumHandle,
    retry_strategy: NativeStructHandle,
) -> Result<ClassDeclarationHandle, BindingError> {
    let destroy_fn = lib
        .declare_native_function("master_destroy")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master to destroy",
        )?
        .return_type(ReturnType::void())?
        .doc(
            doc("Remove and destroy a master.")
                .warning("This method must NOT be called from within the {class:Runtime} thread."),
        )?
        .build()?;

    // Association creation
    let association_class = lib.declare_class("Association")?;

    let event_classes = lib.declare_native_struct("EventClasses")?;
    let event_classes = lib
        .define_native_struct(&event_classes)?
        .add("class1", Type::Bool, "Class 1 events")?
        .add("class2", Type::Bool, "Class 2 events")?
        .add("class3", Type::Bool, "Class 3 events")?
        .doc("Event classes")?
        .build()?;

    let classes = define_classes(lib)?;

    let auto_time_sync_enum = lib
        .define_native_enum("AutoTimeSync")?
        .push("None", "Do not perform automatic timesync")?
        .push(
            "LAN",
            "Perform automatic timesync with Record Current Time (0x18) function code",
        )?
        .push(
            "NonLAN",
            "Perform automatic timesync with Delay Measurement (0x17) function code",
        )?
        .doc("Automatic time synchronization configuration")?
        .build()?;

    let association_configuration = lib.declare_native_struct("AssociationConfiguration")?;
    let association_configuration = lib
        .define_native_struct(&association_configuration)?
        .add(
            "disable_unsol_classes",
            Type::Struct(event_classes.clone()),
            "Classes to disable unsolicited responses at startup",
        )?
        .add(
            "enable_unsol_classes",
            Type::Struct(event_classes),
            "Classes to enable unsolicited responses at startup",
        )?
        .add(
            "startup_integrity_classes",
                Type::Struct(classes),
                doc("Startup integrity classes to ask on master startup and when an outstation restart is detected.").details("For conformance, this should be Class 1230.")
        )?
        .add(
            "auto_time_sync",
            Type::Enum(auto_time_sync_enum),
            "Automatic time sychronization configuration",
        )?
        .add(
            "auto_tasks_retry_strategy",
            Type::Struct(retry_strategy),
            "Automatic tasks retry strategy",
        )?
        .add("keep_alive_timeout",
            Type::Duration(DurationMapping::Seconds),
            doc("Delay of inactivity before sending a REQUEST_LINK_STATUS to the outstation").details("A value of zero means no automatic keep-alives.")
        )?
        .add("auto_integrity_scan_on_buffer_overflow",
            Type::Bool,
            doc("Automatic integrity scan when an EVENT_BUFFER_OVERFLOW is detected")
        )?
        .doc("Association configuration")?
        .build()?;

    let association_handlers = lib.declare_native_struct("AssociationHandlers")?;
    let association_handlers = lib
        .define_native_struct(&association_handlers)?
        .add("integrity_handler", Type::Interface(read_handler.clone()), "Handler for the initial integrity scan")?
        .add("unsolicited_handler", Type::Interface(read_handler.clone()), "Handler for unsolicited responses")?
        .add("default_poll_handler", Type::Interface(read_handler), "Handler for all other responses")?
        .doc(
            doc("Handlers that will receive readings.")
            .details("You can set all handlers to the same handler if knowing what type of event generated the value is not required.")
        )?
        .build()?;

    let time_provider_interface = define_time_provider(lib)?;

    let add_association_fn = lib
        .declare_native_function("master_add_association")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master to add the association to",
        )?
        .param(
            "address",
            Type::Uint16,
            "DNP3 data-link address of the remote outstation",
        )?
        .param(
            "config",
            Type::Struct(association_configuration),
            "Association configuration",
        )?
        .param(
            "handlers",
            Type::Struct(association_handlers),
            "Handlers to call when receiving point data",
        )?
        .param(
            "time_provider",
            Type::Interface(time_provider_interface),
            "Time provider for the association",
        )?
        .return_type(ReturnType::new(
            Type::ClassRef(association_class.clone()),
            "Handle to the created association or NULL if an error occured",
        ))?
        .doc("Add an association to the master")?
        .build()?;

    let set_decode_log_level_fn = lib
        .declare_native_function("master_set_decode_log_level")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master to modify",
        )?
        .param(
            "level",
            Type::Enum(decode_log_level_enum.clone()),
            "Decode log level",
        )?
        .return_type(ReturnType::void())?
        .doc("Set the master decoding level for log messages")?
        .build()?;

    let get_decode_log_level_fn = lib
        .declare_native_function("master_get_decode_log_level")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master to modify",
        )?
        .return_type(ReturnType::new(
            Type::Enum(decode_log_level_enum),
            "Decode log level",
        ))?
        .doc(
            doc("Get the master decoding level for log messages")
                .warning("This cannot be called from within a callback."),
        )?
        .build()?;

    lib.define_class(&master_class)?
        .destructor(&destroy_fn)?
        .method("AddAssociation", &add_association_fn)?
        .method("SetDecodeLogLevel", &set_decode_log_level_fn)?
        .method("GetDecodeLogLevel", &get_decode_log_level_fn)?
        .doc(
            doc("Master channel of communication")
            .details("To communicate with a particular outstation, you need to add an association with {class:Master.AddAssociation()}.")
            .warning("This cannot be called from within a callback.")
        )?
        .build()?;

    Ok(association_class)
}

fn define_time_provider(lib: &mut LibraryBuilder) -> Result<InterfaceHandle, BindingError> {
    let timestamp_struct = lib.declare_native_struct("TimeProviderTimestamp")?;
    let timestamp_struct = lib.define_native_struct(&timestamp_struct)?
        .add("value", Type::Uint64, doc("Value of the timestamp (in milliseconds from UNIX Epoch).").warning("Only 48 bits are available for timestamps."))?
        .add("is_valid", Type::Bool, "True if the timestamp is valid, false otherwise.")?
        .doc(doc("Timestamp value returned by {interface:TimeProvider}.").details("{struct:TimeProviderTimestamp.value} is only valid if {struct:TimeProviderTimestamp.is_valid} is true."))?
        .build()?;

    let valid_constructor = lib
        .declare_native_function("timeprovidertimestamp_valid")?
        .param(
            "value",
            Type::Uint64,
            "Timestamp value in milliseconds from UNIX Epoch",
        )?
        .return_type(ReturnType::new(
            Type::Struct(timestamp_struct.clone()),
            "Timestamp",
        ))?
        .doc("Create a valid timestamp value")?
        .build()?;

    let invalid_constructor = lib
        .declare_native_function("timeprovidertimestamp_invalid")?
        .return_type(ReturnType::new(
            Type::Struct(timestamp_struct.clone()),
            "Timestamp",
        ))?
        .doc("Create an invalid timestamp value")?
        .build()?;

    lib.define_struct(&timestamp_struct)?
        .static_method("valid", &valid_constructor)?
        .static_method("invalid", &invalid_constructor)?
        .build();

    lib.define_interface("TimeProvider", "Current time provider")?
        .callback(
            "get_time",
            doc("Returns the current time of the system.")
                .details("This callback is called when time synchronization is performed.")
                .details(
                    "This can use external clock synchronization or the system clock for example.",
                ),
        )?
        .return_type(ReturnType::new(
            Type::Struct(timestamp_struct),
            "The current time",
        ))?
        .build()?
        .destroy_callback("on_destroy")?
        .build()
}

fn define_classes(lib: &mut LibraryBuilder) -> Result<NativeStructHandle, BindingError> {
    let classes = lib.declare_native_struct("Classes")?;
    let classes = lib
        .define_native_struct(&classes)?
        .add("class0", Type::Bool, "Class 0 (static data)")?
        .add("class1", Type::Bool, "Class 1 events")?
        .add("class2", Type::Bool, "Class 2 events")?
        .add("class3", Type::Bool, "Class 3 events")?
        .doc("Class 0, 1, 2 and 3 config")?
        .build()?;

    let classes_all_fn = lib
        .declare_native_function("classes_all")?
        .return_type(ReturnType::new(Type::Struct(classes.clone()), "Class 1230"))?
        .doc("Class 1230")?
        .build()?;

    let classes_none_fn = lib
        .declare_native_function("classes_none")?
        .return_type(ReturnType::new(Type::Struct(classes.clone()), "No class"))?
        .doc("No class")?
        .build()?;

    lib.define_struct(&classes)?
        .static_method("all", &classes_all_fn)?
        .static_method("none", &classes_none_fn)?
        .build();

    Ok(classes)
}
