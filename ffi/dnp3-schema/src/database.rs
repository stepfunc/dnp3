use class::ClassHandle;
use oo_bindgen::*;

use crate::shared::SharedDefinitions;
use oo_bindgen::name::Name;
use oo_bindgen::structs::{ConstructorType, FunctionArgStructHandle, Number};
use oo_bindgen::types::BasicType;

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

    let update_options = lib.declare_function_arg_struct("UpdateOptions")?;
    let update_options = lib
        .define_function_argument_struct(update_options)?
        .add(
            &update_static,
            BasicType::Bool,
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
        .begin_constructor(
            "init",
            ConstructorType::Normal,
            "Initialize to default values",
        )?
        .default(&update_static, true)?
        .default_variant(&event_mode, "Detect")?
        .end_constructor()?
        .build()?;

    Ok(update_options)
}

fn define_binary_config(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let binary_static_variation = lib
        .define_enum("static_binary_variation")?
        .push("group1_var1", "Binary input - packed format")?
        .push("group1_var2", "Binary input - with flags")?
        .doc("Static binary input variation")?
        .build()?;

    let binary_event_variation = lib
        .define_enum("event_binary_variation")?
        .push("group2_var1", "Binary input event - without time")?
        .push("group2_var2", "Binary input event - with absolute time")?
        .push("group2_var3", "Binary input event - with relative time")?
        .doc("Event binary input variation")?
        .build()?;

    let static_variation = Name::create("static_variation")?;
    let event_variation = Name::create("event_variation")?;

    let config = lib.declare_function_arg_struct("binary_config")?;
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
        .begin_constructor("init", ConstructorType::Normal, "Initialize to defaults")?
        .default_variant(&static_variation, "group1_var1")?
        .default_variant(&event_variation, "group2_var2")?
        .end_constructor()?
        .build()?;

    Ok(config)
}

fn define_double_bit_binary_config(
    lib: &mut LibraryBuilder,
) -> BackTraced<FunctionArgStructHandle> {
    let double_bit_binary_static_variation = lib
        .define_enum("static_double_bit_binary_variation")?
        .push("group3_var1", "Double-bit binary input - packed format")?
        .push("group3_var2", "Double-bit binary input - with flags")?
        .doc("Static double-bit binary input variation")?
        .build()?;

    let double_bit_binary_event_variation = lib
        .define_enum("event_double_bit_binary_variation")?
        .push(
            "group4_var1",
            "Double-bit binary input event - without time",
        )?
        .push(
            "group4_var2",
            "Double-bit binary input event - with absolute time",
        )?
        .push(
            "group4_var3",
            "Double-bit binary input event - with relative time",
        )?
        .doc("Event double-bit binary input variation")?
        .build()?;

    let static_variation = Name::create("static_variation")?;
    let event_variation = Name::create("event_variation")?;

    let config = lib.declare_function_arg_struct("double_bit_binary_config")?;
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
        .begin_constructor("init", ConstructorType::Normal, "Initialize to defaults")?
        .default_variant(&static_variation, "group3_var1")?
        .default_variant(&event_variation, "group4_var2")?
        .end_constructor()?
        .build()?;

    Ok(config)
}

fn define_binary_output_status_config(
    lib: &mut LibraryBuilder,
) -> BackTraced<FunctionArgStructHandle> {
    let binary_output_status_static_variation = lib
        .define_enum("static_binary_output_status_variation")?
        .push("group10_var1", "Binary output - packed format")?
        .push("group10_var2", "Binary output - output status with flags")?
        .doc("Static binary output status variation")?
        .build()?;

    let binary_output_status_event_variation = lib
        .define_enum("event_binary_output_status_variation")?
        .push("group11_var1", "Binary output event - status without time")?
        .push("group11_var2", "Binary output event - status with time")?
        .doc("Event binary output status variation")?
        .build()?;

    let static_variation = Name::create("static_variation")?;
    let event_variation = Name::create("event_variation")?;

    let config = lib.declare_function_arg_struct("binary_output_status_config")?;
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
        .begin_constructor("init", ConstructorType::Normal, "Initialize to defaults")?
        .default_variant(&static_variation, "group10_var1")?
        .default_variant(&event_variation, "group11_var2")?
        .end_constructor()?
        .build()?;

    Ok(config)
}

fn define_counter_config(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let counter_static_variation = lib
        .define_enum("static_counter_variation")?
        .push("group20_var1", "Counter - 32-bit with flag")?
        .push("group20_var2", "Counter - 16-bit with flag")?
        .push("group20_var5", "Counter - 32-bit without flag")?
        .push("group20_var6", "Counter - 16-bit without flag")?
        .doc("Static counter variation")?
        .build()?;

    let counter_event_variation = lib
        .define_enum("event_counter_variation")?
        .push("group22_var1", "Counter event - 32-bit with flag")?
        .push("group22_var2", "Counter event - 16-bit with flag")?
        .push("group22_var5", "Counter event - 32-bit with flag and time")?
        .push("group22_var6", "Counter event - 16-bit with flag and time")?
        .doc("Event counter variation")?
        .build()?;

    let static_variation = Name::create("static_variation")?;
    let event_variation = Name::create("event_variation")?;
    let deadband = Name::create("deadband")?;

    let config = lib.declare_function_arg_struct("counter_config")?;
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
        .add(&deadband, BasicType::U32, "Deadband value")?
        .doc("Counter configuration")?
        .end_fields()?
        .begin_constructor("init", ConstructorType::Normal, "Initialize to defaults")?
        .default_variant(&static_variation, "group20_var1")?
        .default_variant(&event_variation, "group22_var5")?
        .default(&deadband, Number::U32(0))?
        .end_constructor()?
        .build()?;

    Ok(config)
}

fn define_frozen_counter_config(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let frozen_counter_static_variation = lib
        .define_enum("static_frozen_counter_variation")?
        .push("group21_var1", "Frozen Counter - 32-bit with flag")?
        .push("group21_var2", "Frozen Counter - 16-bit with flag")?
        .push("group21_var5", "Frozen Counter - 32-bit with flag and time")?
        .push("group21_var6", "Frozen Counter - 16-bit with flag and time")?
        .push("group21_var9", "Frozen Counter - 32-bit without flag")?
        .push("group21_var10", "Frozen Counter - 16-bit without flag")?
        .doc("Static frozen counter variation")?
        .build()?;

    let frozen_counter_event_variation = lib
        .define_enum("event_frozen_counter_variation")?
        .push("group23_var1", "Frozen Counter event - 32-bit with flag")?
        .push("group23_var2", "Frozen Counter event - 16-bit with flag")?
        .push(
            "group23_var5",
            "Frozen Counter event - 32-bit with flag and time",
        )?
        .push(
            "group23_var6",
            "Frozen Counter event - 16-bit with flag and time",
        )?
        .doc("Event frozen counter variation")?
        .build()?;

    let static_variation = Name::create("static_variation")?;
    let event_variation = Name::create("event_variation")?;
    let deadband = Name::create("deadband")?;

    let config = lib.declare_function_arg_struct("frozen_counter_config")?;
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
        .add(&deadband, BasicType::U32, "Deadband value")?
        .doc("Frozen Counter configuration")?
        .end_fields()?
        .begin_constructor("init", ConstructorType::Normal, "Initialize to defaults")?
        .default_variant(&static_variation, "group21_var1")?
        .default_variant(&event_variation, "group23_var5")?
        .default(&deadband, Number::U32(0))?
        .end_constructor()?
        .build()?;

    Ok(config)
}

pub fn define_analog_config(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let analog_static_variation = lib
        .define_enum("static_analog_variation")?
        .push("group30_var1", "Analog input - 32-bit with flag")?
        .push("group30_var2", "Analog input - 16-bit with flag")?
        .push("group30_var3", "Analog input - 32-bit without flag")?
        .push("group30_var4", "Analog input - 16-bit without flag")?
        .push(
            "Group30Var5",
            "Analog input - single-precision, floating-point with flag",
        )?
        .push(
            "Group30Var6",
            "Analog input - double-precision, floating-point with flag",
        )?
        .doc("Static analog variation")?
        .build()?;

    let analog_event_variation = lib
        .define_enum("event_analog_variation")?
        .push("group32_var1", "Analog input event - 32-bit without time")?
        .push("group32_var2", "Analog input event - 16-bit without time")?
        .push("group32_var3", "Analog input event - 32-bit with time")?
        .push("group32_var4", "Analog input event - 16-bit with time")?
        .push(
            "group32_var5",
            "Analog input event - single-precision, floating-point without time",
        )?
        .push(
            "group32_var6",
            "Analog input event - double-precision, floating-point without time",
        )?
        .push(
            "group32_var7",
            "Analog input event - single-precision, floating-point with time",
        )?
        .push(
            "group32_var8",
            "Analog input event - double-precision, floating-point with time",
        )?
        .doc("Event analog variation")?
        .build()?;

    let static_variation = Name::create("static_variation")?;
    let event_variation = Name::create("event_variation")?;
    let deadband = Name::create("deadband")?;

    let analog_config = lib.declare_function_arg_struct("analog_config")?;
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
        .add(&deadband, BasicType::Double64, "Deadband value")?
        .doc("Analog configuration")?
        .end_fields()?
        .begin_constructor("init", ConstructorType::Normal, "Initialize to defaults")?
        .default_variant(&static_variation, "group30_var1")?
        .default_variant(&event_variation, "group32_var3")?
        .default(&deadband, Number::Double(0.0))?
        .end_constructor()?
        .build()?;

    Ok(config)
}

fn define_analog_output_status_config(
    lib: &mut LibraryBuilder,
) -> BackTraced<FunctionArgStructHandle> {
    let analog_output_status_static_variation = lib
        .define_enum("static_analog_output_status_variation")?
        .push("group40_var1", "Analog output status - 32-bit with flag")?
        .push("group40_var2", "Analog output status - 16-bit with flag")?
        .push(
            "group40_var3",
            "Analog output status - single-precision, floating-point with flag",
        )?
        .push(
            "group40_var4",
            "Analog output status - double-precision, floating-point with flag",
        )?
        .doc("Static analog output status variation")?
        .build()?;

    let analog_output_status_event_variation = lib
        .define_enum("event_analog_output_status_variation")?
        .push("group42_var1", "Analog output event - 32-bit without time")?
        .push("group42_var2", "Analog output event - 16-bit without time")?
        .push("group42_var3", "Analog output event - 32-bit with time")?
        .push("group42_var4", "Analog output event - 16-bit with time")?
        .push(
            "group42_var5",
            "Analog output event - single-precision, floating-point without time",
        )?
        .push(
            "group42_var6",
            "Analog output event - double-precision, floating-point without time",
        )?
        .push(
            "group42_var7",
            "Analog output event - single-precision, floating-point with time",
        )?
        .push(
            "group42_var8",
            "Analog output event - double-precision, floating-point with time",
        )?
        .doc("Event analog output status variation")?
        .build()?;

    let static_variation = Name::create("static_variation")?;
    let event_variation = Name::create("event_variation")?;
    let deadband = Name::create("deadband")?;

    let config = lib.declare_function_arg_struct("analog_output_status_config")?;
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
        .add(&deadband, BasicType::Double64, "Deadband value")?
        .doc("Analog Output Status configuration")?
        .end_fields()?
        .begin_constructor("init", ConstructorType::Normal, "Initialize to defaults")?
        .default_variant(&static_variation, "group40_var1")?
        .default_variant(&event_variation, "group42_var3")?
        .default(&deadband, Number::Double(0.0))?
        .end_constructor()?
        .build()?;

    Ok(config)
}

pub fn define(lib: &mut LibraryBuilder, shared_def: &SharedDefinitions) -> BackTraced<ClassHandle> {
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

    let binary_add_fn = lib
        .define_function("database_add_binary")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point")?
        .param("point_class", event_class.clone(), "Event class")?
        .param("config", binary_config, "Configuration")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully added, false otherwise",
        )?
        .doc("Add a new Binary Input point")?
        .build()?;

    let binary_remove_fn = lib
        .define_function("database_remove_binary")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully removed, false otherwise",
        )?
        .doc("Remove a Binary Input point")?
        .build()?;

    let binary_update_fn = lib
        .define_function("database_update_binary")?
        .param("db", database.clone(), "Database")?
        .param(
            "value",
            shared_def.binary_point.clone(),
            "New value of the point",
        )?
        .param("options", update_options.clone(), "Update options")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully updated, false otherwise",
        )?
        .doc("Update a Binary Input point")?
        .build()?;

    let binary_get_fn = lib
        .define_function("database_get_binary")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point to get")?
        .returns(shared_def.binary_point.clone(), "Binary Input point")?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Binary Input point")?
        .build()?;

    // Double-bit Binary Input
    let double_bit_binary_config = define_double_bit_binary_config(lib)?;

    let double_bit_binary_add_fn = lib
        .define_function("database_add_double_bit_binary")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point")?
        .param("point_class", event_class.clone(), "Event class")?
        .param("config", double_bit_binary_config, "Configuration")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully added, false otherwise",
        )?
        .doc("Add a new Double-Bit Binary Input point")?
        .build()?;

    let double_bit_binary_remove_fn = lib
        .define_function("database_remove_double_bit_binary")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully removed, false otherwise",
        )?
        .doc("Remove a Double-Bit Binary Input point")?
        .build()?;

    let double_bit_binary_update_fn = lib
        .define_function("database_update_double_bit_binary")?
        .param("db", database.clone(), "Database")?
        .param(
            "value",
            shared_def.double_bit_binary_point.clone(),
            "New value of the point",
        )?
        .param("options", update_options.clone(), "Update options")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully updated, false otherwise",
        )?
        .doc("Update a Double-Bit Binary Input point")?
        .build()?;

    let double_bit_binary_get_fn = lib
        .define_function("database_get_double_bit_binary")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point to get")?
        .returns(
            shared_def.double_bit_binary_point.clone(),
            "Double-Bit Binary Input point",
        )?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Double-Bit Binary Input point")?
        .build()?;

    // Binary Output Status
    let binary_output_status_config = define_binary_output_status_config(lib)?;

    let binary_output_status_add_fn = lib
        .define_function("database_add_binary_output_status")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point")?
        .param("point_class", event_class.clone(), "Event class")?
        .param("config", binary_output_status_config, "Configuration")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully added, false otherwise",
        )?
        .doc("Add a new Binary Output Status point")?
        .build()?;

    let binary_output_status_remove_fn = lib
        .define_function("database_remove_binary_output_status")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully removed, false otherwise",
        )?
        .doc("Remove a Binary Output Status point")?
        .build()?;

    let binary_output_status_update_fn = lib
        .define_function("database_update_binary_output_status")?
        .param("db", database.clone(), "Database")?
        .param(
            "value",
            shared_def.binary_output_status_point.clone(),
            "New value of the point",
        )?
        .param("options", update_options.clone(), "Update options")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully updated, false otherwise",
        )?
        .doc("Update a Binary Output Status point")?
        .build()?;

    let binary_output_status_get_fn = lib
        .define_function("database_get_binary_output_status")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point to get")?
        .returns(
            shared_def.binary_output_status_point.clone(),
            "Binary Output Status point",
        )?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Binary Output Status point")?
        .build()?;

    // Counter
    let counter_config = define_counter_config(lib)?;

    let counter_add_fn = lib
        .define_function("database_add_counter")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point")?
        .param("point_class", event_class.clone(), "Event class")?
        .param("config", counter_config, "Configuration")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully added, false otherwise",
        )?
        .doc("Add a new Counter point")?
        .build()?;

    let counter_remove_fn = lib
        .define_function("database_remove_counter")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully removed, false otherwise",
        )?
        .doc("Remove a Counter point")?
        .build()?;

    let counter_update_fn = lib
        .define_function("database_update_counter")?
        .param("db", database.clone(), "Database")?
        .param(
            "value",
            shared_def.counter_point.clone(),
            "New value of the point",
        )?
        .param("options", update_options.clone(), "Update options")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully updated, false otherwise",
        )?
        .doc("Update a Counter point")?
        .build()?;

    let counter_get_fn = lib
        .define_function("database_get_counter")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point to get")?
        .returns(shared_def.counter_point.clone(), "Counter point")?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Counter point")?
        .build()?;

    // Frozen Counter
    let frozen_counter_config = define_frozen_counter_config(lib)?;

    let frozen_counter_add_fn = lib
        .define_function("database_add_frozen_counter")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point")?
        .param("point_class", event_class.clone(), "Event class")?
        .param("config", frozen_counter_config, "Configuration")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully added, false otherwise",
        )?
        .doc("Add a new Frozen Counter point")?
        .build()?;

    let frozen_counter_remove_fn = lib
        .define_function("database_remove_frozen_counter")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully removed, false otherwise",
        )?
        .doc("Remove a Frozen Counter point")?
        .build()?;

    let frozen_counter_update_fn = lib
        .define_function("database_update_frozen_counter")?
        .param("db", database.clone(), "Database")?
        .param(
            "value",
            shared_def.frozen_counter_point.clone(),
            "New value of the point",
        )?
        .param("options", update_options.clone(), "Update options")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully updated, false otherwise",
        )?
        .doc("Update an Frozen Counter point")?
        .build()?;

    let frozen_counter_get_fn = lib
        .define_function("database_get_frozen_counter")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point to get")?
        .returns(
            shared_def.frozen_counter_point.clone(),
            "Frozen Counter point",
        )?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Frozen Counter point")?
        .build()?;

    // Analog
    let analog_config = define_analog_config(lib)?;

    let analog_add_fn = lib
        .define_function("database_add_analog")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point")?
        .param("point_class", event_class.clone(), "Event class")?
        .param("config", analog_config, "Configuration")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully added, false otherwise",
        )?
        .doc("Add a new Analog point")?
        .build()?;

    let analog_remove_fn = lib
        .define_function("database_remove_analog")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully removed, false otherwise",
        )?
        .doc("Remove an Analog point")?
        .build()?;

    let analog_update_fn = lib
        .define_function("database_update_analog")?
        .param("db", database.clone(), "Database")?
        .param(
            "value",
            shared_def.analog_point.clone(),
            "New value of the point",
        )?
        .param("options", update_options.clone(), "Update options")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully updated, false otherwise",
        )?
        .doc("Update a Analog point")?
        .build()?;

    let analog_get_fn = lib
        .define_function("database_get_analog")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point to get")?
        .returns(shared_def.analog_point.clone(), "Analog point")?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Analog point")?
        .build()?;

    // Analog Output Status
    let analog_output_status_config = define_analog_output_status_config(lib)?;
    let analog_output_status_add_fn = lib
        .define_function("database_add_analog_output_status")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point")?
        .param("point_class", event_class.clone(), "Event class")?
        .param("config", analog_output_status_config, "Configuration")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully added, false otherwise",
        )?
        .doc("Add a new Analog Output Status point")?
        .build()?;

    let analog_output_status_remove_fn = lib
        .define_function("database_remove_analog_output_status")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully removed, false otherwise",
        )?
        .doc("Remove an Analog Output Status point")?
        .build()?;

    let analog_output_status_update_fn = lib
        .define_function("database_update_analog_output_status")?
        .param("db", database.clone(), "Database")?
        .param(
            "value",
            shared_def.analog_output_status_point.clone(),
            "New value of the point",
        )?
        .param("options", update_options.clone(), "Update options")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully updated, false otherwise",
        )?
        .doc("Update a Analog Output Status point")?
        .build()?;

    let analog_output_status_get_fn = lib
        .define_function("database_get_analog_output_status")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point to get")?
        .returns(
            shared_def.analog_output_status_point.clone(),
            "Analog Output Status point",
        )?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Analog Output Status point")?
        .build()?;

    // Octet String
    let octet_string = lib.define_collection("octet_string", BasicType::U8, false)?;

    let octet_string_add_fn = lib
        .define_function("database_add_octet_string")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point")?
        .param("point_class", event_class, "Event class")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully added, false otherwise",
        )?
        .doc("Add a new Octet String point")?
        .build()?;

    let octet_string_remove_fn = lib
        .define_function("database_remove_octet_string")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the point")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully removed, false otherwise",
        )?
        .doc("Remove an Octet String point")?
        .build()?;

    let octet_string_update_fn = lib
        .define_function("database_update_octet_string")?
        .param("db", database.clone(), "Database")?
        .param("index", BasicType::U16, "Index of the octet string")?
        .param("value", octet_string, "New value of the point")?
        .param("options", update_options, "Update options")?
        .returns(
            BasicType::Bool,
            "True if the point was successfully updated, false otherwise",
        )?
        .doc("Update an Octet String point")?
        .build()?;

    // TODO: Add a getter for octet strings

    let database = lib
        .define_class(&database)?
        .method("add_binary", &binary_add_fn)?
        .method("remove_binary", &binary_remove_fn)?
        .method("update_binary", &binary_update_fn)?
        .method("get_binary", &binary_get_fn)?
        .method("add_double_bit_binary", &double_bit_binary_add_fn)?
        .method("remove_double_bit_binary", &double_bit_binary_remove_fn)?
        .method("update_double_bit_binary", &double_bit_binary_update_fn)?
        .method("get_double_bit_binary", &double_bit_binary_get_fn)?
        .method("add_binary_output_status", &binary_output_status_add_fn)?
        .method(
            "remove_binary_output_status",
            &binary_output_status_remove_fn,
        )?
        .method(
            "update_binary_output_status",
            &binary_output_status_update_fn,
        )?
        .method("get_binary_output_status", &binary_output_status_get_fn)?
        .method("add_counter", &counter_add_fn)?
        .method("remove_counter", &counter_remove_fn)?
        .method("update_counter", &counter_update_fn)?
        .method("get_counter", &counter_get_fn)?
        .method("add_frozen_counter", &frozen_counter_add_fn)?
        .method("remove_frozen_counter", &frozen_counter_remove_fn)?
        .method("update_frozen_counter", &frozen_counter_update_fn)?
        .method("get_frozen_counter", &frozen_counter_get_fn)?
        .method("add_analog", &analog_add_fn)?
        .method("remove_analog", &analog_remove_fn)?
        .method("update_analog", &analog_update_fn)?
        .method("get_analog", &analog_get_fn)?
        .method("add_analog_output_status", &analog_output_status_add_fn)?
        .method(
            "remove_analog_output_status",
            &analog_output_status_remove_fn,
        )?
        .method(
            "update_analog_output_status",
            &analog_output_status_update_fn,
        )?
        .method("get_analog_output_status", &analog_output_status_get_fn)?
        .method("add_octet_string", &octet_string_add_fn)?
        .method("remove_octet_string", &octet_string_remove_fn)?
        .method("update_octet_string", &octet_string_update_fn)?
        .doc(
            doc("Internal database access")
                .warning("This object is only valid within the transaction."),
        )?
        .build()?;

    Ok(database)
}
