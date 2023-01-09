use crate::gv;
use crate::shared::SharedDefinitions;
use oo_bindgen::model::*;

fn define_update_options(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let event_mode_enum = lib
        .define_enum("event_mode")?
        .push(
            "detect",
            doc("Detect events in a type dependent fashion")
                .details("This is the default mode that should be used."),
        )?
        .push(
            "force",
            "Produce an event whether the value has changed or not",
        )?
        .push("suppress", "Never produce an event regardless of change")?
        .doc("Controls how events are processed when updating values in the database.")?
        .build()?;

    let update_static = Name::create("update_static")?;
    let event_mode = Name::create("event_mode")?;

    let update_options = lib.declare_function_argument_struct("update_options")?;
    let update_options = lib
        .define_function_argument_struct(update_options)?
        .add(
            &update_static,
            Primitive::Bool,
            "Optionally bypass updating the static database (the current value)",
        )?
        .add(
            &event_mode,
            event_mode_enum,
            "Determines how/if an event is produced",
        )?
        .doc(
            doc("Options that control how the update is performed.")
                .details("99% of the time, the default value should be used."),
        )?
        .end_fields()?
        .begin_initializer(
            "detect_event",
            InitializerType::Static,
            "Default event detection mode. Updates the static value and automatically detects event.",
        )?
        .default(&update_static, true)?
        .default_variant(&event_mode, "detect")?
        .end_initializer()?
        .begin_initializer(
            "no_event",
            InitializerType::Static,
            "Only update the static value. Usefull during initialization of the database.",
        )?
        .default(&update_static, true)?
        .default_variant(&event_mode, "suppress")?
        .end_initializer()?
        .build()?;

    Ok(update_options)
}

fn define_binary_config(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let binary_static_variation = lib
        .define_enum("static_binary_input_variation")?
        .push(gv(1, 1), "Binary input - packed format")?
        .push(gv(1, 2), "Binary input - with flags")?
        .doc("Static binary input variation")?
        .build()?;

    let binary_event_variation = lib
        .define_enum("event_binary_input_variation")?
        .push(gv(2, 1), "Binary input event - without time")?
        .push(gv(2, 2), "Binary input event - with absolute time")?
        .push(gv(2, 3), "Binary input event - with relative time")?
        .doc("Event binary input variation")?
        .build()?;

    let static_variation = Name::create("static_variation")?;
    let event_variation = Name::create("event_variation")?;

    let config = lib.declare_function_argument_struct("binary_input_config")?;
    let config = lib
        .define_function_argument_struct(config)?
        .add(
            &static_variation,
            binary_static_variation,
            "Default static variation",
        )?
        .add(
            &event_variation,
            binary_event_variation,
            "Default event variation",
        )?
        .doc("Binary Input configuration")?
        .end_fields()?
        .add_full_initializer("create")?
        .begin_initializer("init", InitializerType::Normal, "Initialize to defaults")?
        .default_variant(&static_variation, gv(1, 1))?
        .default_variant(&event_variation, gv(2, 1))?
        .end_initializer()?
        .build()?;

    Ok(config)
}

fn define_double_bit_binary_config(
    lib: &mut LibraryBuilder,
) -> BackTraced<FunctionArgStructHandle> {
    let double_bit_binary_static_variation = lib
        .define_enum("static_double_bit_binary_input_variation")?
        .push(gv(3, 1), "Double-bit binary input - packed format")?
        .push(gv(3, 2), "Double-bit binary input - with flags")?
        .doc("Static double-bit binary input variation")?
        .build()?;

    let double_bit_binary_event_variation = lib
        .define_enum("event_double_bit_binary_input_variation")?
        .push(gv(4, 1), "Double-bit binary input event - without time")?
        .push(
            gv(4, 2),
            "Double-bit binary input event - with absolute time",
        )?
        .push(
            gv(4, 3),
            "Double-bit binary input event - with relative time",
        )?
        .doc("Event double-bit binary input variation")?
        .build()?;

    let static_variation = Name::create("static_variation")?;
    let event_variation = Name::create("event_variation")?;

    let config = lib.declare_function_argument_struct("double_bit_binary_input_config")?;
    let config = lib
        .define_function_argument_struct(config)?
        .add(
            &static_variation,
            double_bit_binary_static_variation,
            "Default static variation",
        )?
        .add(
            &event_variation,
            double_bit_binary_event_variation,
            "Default event variation",
        )?
        .doc("Double-Bit Binary Input configuration")?
        .end_fields()?
        .add_full_initializer("create")?
        .begin_initializer("init", InitializerType::Normal, "Initialize to defaults")?
        .default_variant(&static_variation, gv(3, 1))?
        .default_variant(&event_variation, gv(4, 1))?
        .end_initializer()?
        .build()?;

    Ok(config)
}

fn define_binary_output_status_config(
    lib: &mut LibraryBuilder,
) -> BackTraced<FunctionArgStructHandle> {
    let binary_output_status_static_variation = lib
        .define_enum("static_binary_output_status_variation")?
        .push(gv(10, 1), "Binary output - packed format")?
        .push(gv(10, 2), "Binary output - output status with flags")?
        .doc("Static binary output status variation")?
        .build()?;

    let binary_output_status_event_variation = lib
        .define_enum("event_binary_output_status_variation")?
        .push(gv(11, 1), "Binary output event - status without time")?
        .push(gv(11, 2), "Binary output event - status with time")?
        .doc("Event binary output status variation")?
        .build()?;

    let static_variation = Name::create("static_variation")?;
    let event_variation = Name::create("event_variation")?;

    let config = lib.declare_function_argument_struct("binary_output_status_config")?;
    let config = lib
        .define_function_argument_struct(config)?
        .add(
            &static_variation,
            binary_output_status_static_variation,
            "Default static variation",
        )?
        .add(
            &event_variation,
            binary_output_status_event_variation,
            "Default event variation",
        )?
        .doc("Binary Output Status configuration")?
        .end_fields()?
        .add_full_initializer("create")?
        .begin_initializer("init", InitializerType::Normal, "Initialize to defaults")?
        .default_variant(&static_variation, gv(10, 1))?
        .default_variant(&event_variation, gv(11, 2))?
        .end_initializer()?
        .build()?;

    Ok(config)
}

fn define_counter_config(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let counter_static_variation = lib
        .define_enum("static_counter_variation")?
        .push(gv(20, 1), "Counter - 32-bit with flag")?
        .push(gv(20, 2), "Counter - 16-bit with flag")?
        .push(gv(20, 5), "Counter - 32-bit without flag")?
        .push(gv(20, 6), "Counter - 16-bit without flag")?
        .doc("Static counter variation")?
        .build()?;

    let counter_event_variation = lib
        .define_enum("event_counter_variation")?
        .push(gv(22, 1), "Counter event - 32-bit with flag")?
        .push(gv(22, 2), "Counter event - 16-bit with flag")?
        .push(gv(22, 5), "Counter event - 32-bit with flag and time")?
        .push(gv(22, 6), "Counter event - 16-bit with flag and time")?
        .doc("Event counter variation")?
        .build()?;

    let static_variation = Name::create("static_variation")?;
    let event_variation = Name::create("event_variation")?;
    let deadband = Name::create("deadband")?;

    let config = lib.declare_function_argument_struct("counter_config")?;
    let config = lib
        .define_function_argument_struct(config)?
        .add(
            &static_variation,
            counter_static_variation,
            "Default static variation",
        )?
        .add(
            &event_variation,
            counter_event_variation,
            "Default event variation",
        )?
        .add(&deadband, Primitive::U32, "Deadband value")?
        .doc("Counter configuration")?
        .end_fields()?
        .add_full_initializer("create")?
        .begin_initializer("init", InitializerType::Normal, "Initialize to defaults")?
        .default_variant(&static_variation, gv(20, 1))?
        .default_variant(&event_variation, gv(22, 1))?
        .default(&deadband, NumberValue::U32(0))?
        .end_initializer()?
        .build()?;

    Ok(config)
}

fn define_frozen_counter_config(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let frozen_counter_static_variation = lib
        .define_enum("static_frozen_counter_variation")?
        .push(gv(21, 1), "Frozen Counter - 32-bit with flag")?
        .push(gv(21, 2), "Frozen Counter - 16-bit with flag")?
        .push(gv(21, 5), "Frozen Counter - 32-bit with flag and time")?
        .push(gv(21, 6), "Frozen Counter - 16-bit with flag and time")?
        .push(gv(21, 9), "Frozen Counter - 32-bit without flag")?
        .push(gv(21, 10), "Frozen Counter - 16-bit without flag")?
        .doc("Static frozen counter variation")?
        .build()?;

    let frozen_counter_event_variation = lib
        .define_enum("event_frozen_counter_variation")?
        .push(gv(23, 1), "Frozen Counter event - 32-bit with flag")?
        .push(gv(23, 2), "Frozen Counter event - 16-bit with flag")?
        .push(
            gv(23, 5),
            "Frozen Counter event - 32-bit with flag and time",
        )?
        .push(
            gv(23, 6),
            "Frozen Counter event - 16-bit with flag and time",
        )?
        .doc("Event frozen counter variation")?
        .build()?;

    let static_variation = Name::create("static_variation")?;
    let event_variation = Name::create("event_variation")?;
    let deadband = Name::create("deadband")?;

    let config = lib.declare_function_argument_struct("frozen_counter_config")?;
    let config = lib
        .define_function_argument_struct(config)?
        .add(
            &static_variation,
            frozen_counter_static_variation,
            "Default static variation",
        )?
        .add(
            &event_variation,
            frozen_counter_event_variation,
            "Default event variation",
        )?
        .add(&deadband, Primitive::U32, "Deadband value")?
        .doc("Frozen Counter configuration")?
        .end_fields()?
        .add_full_initializer("create")?
        .begin_initializer("init", InitializerType::Normal, "Initialize to defaults")?
        .default_variant(&static_variation, gv(21, 1))?
        .default_variant(&event_variation, gv(23, 1))?
        .default(&deadband, NumberValue::U32(0))?
        .end_initializer()?
        .build()?;

    Ok(config)
}

pub fn define_analog_config(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let analog_static_variation = lib
        .define_enum("static_analog_input_variation")?
        .push(gv(30, 1), "Analog input - 32-bit with flag")?
        .push(gv(30, 2), "Analog input - 16-bit with flag")?
        .push(gv(30, 3), "Analog input - 32-bit without flag")?
        .push(gv(30, 4), "Analog input - 16-bit without flag")?
        .push(
            gv(30, 5),
            "Analog input - single-precision, floating-point with flag",
        )?
        .push(
            gv(30, 6),
            "Analog input - double-precision, floating-point with flag",
        )?
        .doc("Static analog variation")?
        .build()?;

    let analog_event_variation = lib
        .define_enum("event_analog_input_variation")?
        .push(gv(32, 1), "Analog input event - 32-bit without time")?
        .push(gv(32, 2), "Analog input event - 16-bit without time")?
        .push(gv(32, 3), "Analog input event - 32-bit with time")?
        .push(gv(32, 4), "Analog input event - 16-bit with time")?
        .push(
            gv(32, 5),
            "Analog input event - single-precision, floating-point without time",
        )?
        .push(
            gv(32, 6),
            "Analog input event - double-precision, floating-point without time",
        )?
        .push(
            gv(32, 7),
            "Analog input event - single-precision, floating-point with time",
        )?
        .push(
            gv(32, 8),
            "Analog input event - double-precision, floating-point with time",
        )?
        .doc("Event analog variation")?
        .build()?;

    let static_variation = Name::create("static_variation")?;
    let event_variation = Name::create("event_variation")?;
    let deadband = Name::create("deadband")?;

    let analog_config = lib.declare_function_argument_struct("analog_input_config")?;
    let config = lib
        .define_function_argument_struct(analog_config)?
        .add(
            &static_variation,
            analog_static_variation,
            "Default static variation",
        )?
        .add(
            &event_variation,
            analog_event_variation,
            "Default event variation",
        )?
        .add(&deadband, Primitive::Double, "Deadband value")?
        .doc("Analog configuration")?
        .end_fields()?
        .add_full_initializer("create")?
        .begin_initializer("init", InitializerType::Normal, "Initialize to defaults")?
        .default_variant(&static_variation, gv(30, 1))?
        .default_variant(&event_variation, gv(32, 1))?
        .default(&deadband, NumberValue::Double(0.0))?
        .end_initializer()?
        .build()?;

    Ok(config)
}

fn define_analog_output_status_config(
    lib: &mut LibraryBuilder,
) -> BackTraced<FunctionArgStructHandle> {
    let analog_output_status_static_variation = lib
        .define_enum("static_analog_output_status_variation")?
        .push(gv(40, 1), "Analog output status - 32-bit with flag")?
        .push(gv(40, 2), "Analog output status - 16-bit with flag")?
        .push(
            gv(40, 3),
            "Analog output status - single-precision, floating-point with flag",
        )?
        .push(
            gv(40, 4),
            "Analog output status - double-precision, floating-point with flag",
        )?
        .doc("Static analog output status variation")?
        .build()?;

    let analog_output_status_event_variation = lib
        .define_enum("event_analog_output_status_variation")?
        .push(gv(42, 1), "Analog output event - 32-bit without time")?
        .push(gv(42, 2), "Analog output event - 16-bit without time")?
        .push(gv(42, 3), "Analog output event - 32-bit with time")?
        .push(gv(42, 4), "Analog output event - 16-bit with time")?
        .push(
            gv(42, 5),
            "Analog output event - single-precision, floating-point without time",
        )?
        .push(
            gv(42, 6),
            "Analog output event - double-precision, floating-point without time",
        )?
        .push(
            gv(42, 7),
            "Analog output event - single-precision, floating-point with time",
        )?
        .push(
            gv(42, 8),
            "Analog output event - double-precision, floating-point with time",
        )?
        .doc("Event analog output status variation")?
        .build()?;

    let static_variation = Name::create("static_variation")?;
    let event_variation = Name::create("event_variation")?;
    let deadband = Name::create("deadband")?;

    let config = lib.declare_function_argument_struct("analog_output_status_config")?;
    let config = lib
        .define_function_argument_struct(config)?
        .add(
            &static_variation,
            analog_output_status_static_variation,
            "Default static variation",
        )?
        .add(
            &event_variation,
            analog_output_status_event_variation,
            "Default event variation",
        )?
        .add(&deadband, Primitive::Double, "Deadband value")?
        .doc("Analog Output Status configuration")?
        .end_fields()?
        .add_full_initializer("create")?
        .begin_initializer("init", InitializerType::Normal, "Initialize to defaults")?
        .default_variant(&static_variation, gv(40, 1))?
        .default_variant(&event_variation, gv(42, 1))?
        .default(&deadband, NumberValue::Double(0.0))?
        .end_initializer()?
        .build()?;

    Ok(config)
}

pub(crate) struct DatabaseTypes {
    pub(crate) database_transaction: SynchronousInterface,
    pub(crate) database_handle: ClassHandle,
}

pub(crate) fn define(
    lib: &mut LibraryBuilder,
    shared_def: &SharedDefinitions,
) -> BackTraced<DatabaseTypes> {
    let database = define_database(lib, shared_def)?;

    let database_transaction = lib
        .define_interface("database_transaction", "Database transaction interface")?
        .begin_callback("execute", "Execute a transaction on the provided database")?
        .param("database", database.declaration(), "Database")?
        .enable_functional_transform()
        .end_callback()?
        .build_sync()?;

    let database_handle = lib.declare_class("database_handle")?;

    let transaction_method = lib
        .define_method("transaction", database_handle.clone())?
        .param(
            "callback",
            database_transaction.clone(),
            "callback interface",
        )?
        .doc("Acquire a mutex on the underlying database and apply a set of changes as a transaction")?
        .build()?;

    let database_handle = lib
        .define_class(&database_handle)?
        .method(transaction_method)?
        .doc(
            doc("Handle typed used to perform transactions on the database inside of control and freeze callbacks")
                .details("This type has the same transaction method as {class:outstation.transaction()} but it is only used in these callbacks.")
        )?
        .build()?;

    Ok(DatabaseTypes {
        database_transaction,
        database_handle,
    })
}

pub(crate) fn define_database(
    lib: &mut LibraryBuilder,
    shared_def: &SharedDefinitions,
) -> BackTraced<ClassHandle> {
    let database = lib.declare_class("database")?;

    let event_class = lib
        .define_enum("event_class")?
        .push("none", "Does not generate events")?
        .push("class1", "Class 1 event")?
        .push("class2", "Class 2 event")?
        .push("class3", "Class 3 event")?
        .doc("Event class")?
        .build()?;

    let update_options = define_update_options(lib)?;

    // Binary Input
    let binary_config = define_binary_config(lib)?;

    let add_binary = lib
        .define_method("add_binary_input", database.clone())?
        .param("index", Primitive::U16, "Index of the point")?
        .param("point_class", event_class.clone(), "Event class")?
        .param("config", binary_config, "Configuration")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully added, false otherwise",
        )?
        .doc("Add a new BinaryInput point")?
        .build()?;

    let remove_binary = lib
        .define_method("remove_binary_input", database.clone())?
        .param("index", Primitive::U16, "Index of the point")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully removed, false otherwise",
        )?
        .doc("Remove a BinaryInput point")?
        .build()?;

    let update_binary = lib
        .define_method("update_binary_input", database.clone())?
        .param(
            "value",
            shared_def.binary_point.clone(),
            "New value of the point",
        )?
        .param("options", update_options.clone(), "Update options")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully updated, false otherwise",
        )?
        .doc("Update a BinaryInput point")?
        .build()?;

    let get_binary = lib
        .define_method("get_binary_input", database.clone())?
        .param("index", Primitive::U16, "Index of the point to get")?
        .returns(shared_def.binary_point.clone(), "Binary Input point")?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a BinaryInput point")?
        .build()?;

    // Double-bit Binary Input
    let double_bit_binary_config = define_double_bit_binary_config(lib)?;

    let add_double_bit_binary = lib
        .define_method("add_double_bit_binary_input", database.clone())?
        .param("index", Primitive::U16, "Index of the point")?
        .param("point_class", event_class.clone(), "Event class")?
        .param("config", double_bit_binary_config, "Configuration")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully added, false otherwise",
        )?
        .doc("Add a new Double-Bit Binary Input point")?
        .build()?;

    let remove_double_bit_binary = lib
        .define_method("remove_double_bit_binary_input", database.clone())?
        .param("index", Primitive::U16, "Index of the point")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully removed, false otherwise",
        )?
        .doc("Remove a Double-Bit Binary Input point")?
        .build()?;

    let update_double_bit_binary = lib
        .define_method("update_double_bit_binary_input", database.clone())?
        .param(
            "value",
            shared_def.double_bit_binary_point.clone(),
            "New value of the point",
        )?
        .param("options", update_options.clone(), "Update options")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully updated, false otherwise",
        )?
        .doc("Update a Double-Bit Binary Input point")?
        .build()?;

    let get_double_bit_binary = lib
        .define_method("get_double_bit_binary_input", database.clone())?
        .param("index", Primitive::U16, "Index of the point to get")?
        .returns(
            shared_def.double_bit_binary_point.clone(),
            "Double-Bit Binary Input point",
        )?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Double-Bit Binary Input point")?
        .build()?;

    // Binary Output Status
    let binary_output_status_config = define_binary_output_status_config(lib)?;

    let add_binary_output_status = lib
        .define_method("add_binary_output_status", database.clone())?
        .param("index", Primitive::U16, "Index of the point")?
        .param("point_class", event_class.clone(), "Event class")?
        .param("config", binary_output_status_config, "Configuration")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully added, false otherwise",
        )?
        .doc("Add a new Binary Output Status point")?
        .build()?;

    let remove_binary_output_status = lib
        .define_method("remove_binary_output_status", database.clone())?
        .param("index", Primitive::U16, "Index of the point")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully removed, false otherwise",
        )?
        .doc("Remove a Binary Output Status point")?
        .build()?;

    let update_binary_output_status = lib
        .define_method("update_binary_output_status", database.clone())?
        .param(
            "value",
            shared_def.binary_output_status_point.clone(),
            "New value of the point",
        )?
        .param("options", update_options.clone(), "Update options")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully updated, false otherwise",
        )?
        .doc("Update a Binary Output Status point")?
        .build()?;

    let get_binary_output_status = lib
        .define_method("get_binary_output_status", database.clone())?
        .param("index", Primitive::U16, "Index of the point to get")?
        .returns(
            shared_def.binary_output_status_point.clone(),
            "Binary Output Status point",
        )?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Binary Output Status point")?
        .build()?;

    // Counter
    let counter_config = define_counter_config(lib)?;

    let add_counter = lib
        .define_method("add_counter", database.clone())?
        .param("index", Primitive::U16, "Index of the point")?
        .param("point_class", event_class.clone(), "Event class")?
        .param("config", counter_config, "Configuration")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully added, false otherwise",
        )?
        .doc("Add a new Counter point")?
        .build()?;

    let remove_counter = lib
        .define_method("remove_counter", database.clone())?
        .param("index", Primitive::U16, "Index of the point")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully removed, false otherwise",
        )?
        .doc("Remove a Counter point")?
        .build()?;

    let update_counter = lib
        .define_method("update_counter", database.clone())?
        .param(
            "value",
            shared_def.counter_point.clone(),
            "New value of the point",
        )?
        .param("options", update_options.clone(), "Update options")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully updated, false otherwise",
        )?
        .doc("Update a Counter point")?
        .build()?;

    let get_counter = lib
        .define_method("get_counter", database.clone())?
        .param("index", Primitive::U16, "Index of the point to get")?
        .returns(shared_def.counter_point.clone(), "Counter point")?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Counter point")?
        .build()?;

    // Frozen Counter
    let frozen_counter_config = define_frozen_counter_config(lib)?;

    let add_frozen_counter = lib
        .define_method("add_frozen_counter", database.clone())?
        .param("index", Primitive::U16, "Index of the point")?
        .param("point_class", event_class.clone(), "Event class")?
        .param("config", frozen_counter_config, "Configuration")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully added, false otherwise",
        )?
        .doc("Add a new Frozen Counter point")?
        .build()?;

    let remove_frozen_counter = lib
        .define_method("remove_frozen_counter", database.clone())?
        .param("index", Primitive::U16, "Index of the point")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully removed, false otherwise",
        )?
        .doc("Remove a Frozen Counter point")?
        .build()?;

    let update_frozen_counter = lib
        .define_method("update_frozen_counter", database.clone())?
        .param(
            "value",
            shared_def.frozen_counter_point.clone(),
            "New value of the point",
        )?
        .param("options", update_options.clone(), "Update options")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully updated, false otherwise",
        )?
        .doc("Update an Frozen Counter point")?
        .build()?;

    let get_frozen_counter = lib
        .define_method("get_frozen_counter", database.clone())?
        .param("index", Primitive::U16, "Index of the point to get")?
        .returns(
            shared_def.frozen_counter_point.clone(),
            "Frozen Counter point",
        )?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Frozen Counter point")?
        .build()?;

    // Analog
    let analog_config = define_analog_config(lib)?;

    let add_analog = lib
        .define_method("add_analog_input", database.clone())?
        .param("index", Primitive::U16, "Index of the point")?
        .param("point_class", event_class.clone(), "Event class")?
        .param("config", analog_config, "Configuration")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully added, false otherwise",
        )?
        .doc("Add a new AnalogInput point")?
        .build()?;

    let remove_analog = lib
        .define_method("remove_analog_input", database.clone())?
        .param("index", Primitive::U16, "Index of the point")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully removed, false otherwise",
        )?
        .doc("Remove an AnalogInput point")?
        .build()?;

    let update_analog = lib
        .define_method("update_analog_input", database.clone())?
        .param(
            "value",
            shared_def.analog_point.clone(),
            "New value of the point",
        )?
        .param("options", update_options.clone(), "Update options")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully updated, false otherwise",
        )?
        .doc("Update a AnalogInput point")?
        .build()?;

    let get_analog = lib
        .define_method("get_analog_input", database.clone())?
        .param("index", Primitive::U16, "Index of the point to get")?
        .returns(shared_def.analog_point.clone(), "Analog point")?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a AnalogInput point")?
        .build()?;

    // Analog Output Status
    let analog_output_status_config = define_analog_output_status_config(lib)?;
    let add_analog_output_status = lib
        .define_method("add_analog_output_status", database.clone())?
        .param("index", Primitive::U16, "Index of the point")?
        .param("point_class", event_class.clone(), "Event class")?
        .param("config", analog_output_status_config, "Configuration")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully added, false otherwise",
        )?
        .doc("Add a new Analog Output Status point")?
        .build()?;

    let remove_analog_output_status = lib
        .define_method("remove_analog_output_status", database.clone())?
        .param("index", Primitive::U16, "Index of the point")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully removed, false otherwise",
        )?
        .doc("Remove an Analog Output Status point")?
        .build()?;

    let update_analog_output_status = lib
        .define_method("update_analog_output_status", database.clone())?
        .param(
            "value",
            shared_def.analog_output_status_point.clone(),
            "New value of the point",
        )?
        .param("options", update_options.clone(), "Update options")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully updated, false otherwise",
        )?
        .doc("Update a Analog Output Status point")?
        .build()?;

    let get_analog_output_status = lib
        .define_method("get_analog_output_status", database.clone())?
        .param("index", Primitive::U16, "Index of the point to get")?
        .returns(
            shared_def.analog_output_status_point.clone(),
            "Analog Output Status point",
        )?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Analog Output Status point")?
        .build()?;

    // Octet String
    let octet_string = lib.define_collection("octet_string_value", Primitive::U8, false)?;

    let add_octet_string = lib
        .define_method("add_octet_string", database.clone())?
        .param("index", Primitive::U16, "Index of the point")?
        .param("point_class", event_class, "Event class")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully added, false otherwise",
        )?
        .doc("Add a new Octet String point")?
        .build()?;

    let remove_octet_string = lib
        .define_method("remove_octet_string", database.clone())?
        .param("index", Primitive::U16, "Index of the point")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully removed, false otherwise",
        )?
        .doc("Remove an Octet String point")?
        .build()?;

    let update_octet_string = lib
        .define_method("update_octet_string", database.clone())?
        .param("index", Primitive::U16, "Index of the octet string")?
        .param("value", octet_string, "New value of the point")?
        .param("options", update_options, "Update options")?
        .returns(
            Primitive::Bool,
            "True if the point was successfully updated, false otherwise",
        )?
        .doc("Update an Octet String point")?
        .build()?;

    let attr_error = lib
        .define_error_type(
            "attr_error",
            "attr_exception",
            ExceptionType::UncheckedException,
        )?
        .doc("Errors that can occur when defining attributes")?
        .add_error(
            "not_writable",
            "This attribute cannot be configured as writable",
        )?
        .build()?;

    let define_string_attr = lib
        .define_method("define_string_attr", database.clone())?
        .doc("Define a string attribute")?
        .param(
            "set",
            Primitive::U8,
            "The set to which the attribute belongs",
        )?
        .param("variation", Primitive::U8, "The variation of the attribute")?
        .param("value", StringType, "The value of the attribute")?
        .fails_with(attr_error)?
        .build()?;

    // TODO: Add a getter for octet strings

    let database = lib
        .define_class(&database)?
        // binary methods
        .method(add_binary)?
        .method(remove_binary)?
        .method(update_binary)?
        .method(get_binary)?
        // double-bit binary methods
        .method(add_double_bit_binary)?
        .method(remove_double_bit_binary)?
        .method(update_double_bit_binary)?
        .method(get_double_bit_binary)?
        // binary output status methods
        .method(add_binary_output_status)?
        .method(remove_binary_output_status)?
        .method(update_binary_output_status)?
        .method(get_binary_output_status)?
        // counter methods
        .method(add_counter)?
        .method(remove_counter)?
        .method(update_counter)?
        .method(get_counter)?
        // frozen-counter methods
        .method(add_frozen_counter)?
        .method(remove_frozen_counter)?
        .method(update_frozen_counter)?
        .method(get_frozen_counter)?
        // analog methods
        .method(add_analog)?
        .method(remove_analog)?
        .method(update_analog)?
        .method(get_analog)?
        // analog output status methods
        .method(add_analog_output_status)?
        .method(remove_analog_output_status)?
        .method(update_analog_output_status)?
        .method(get_analog_output_status)?
        // octet-string methods
        .method(add_octet_string)?
        .method(remove_octet_string)?
        .method(update_octet_string)?
        // device attributes
        .method(define_string_attr)?
        .doc(
            doc("Internal database access")
                .warning("This object is only valid within a transaction"),
        )?
        .build()?;

    Ok(database)
}
