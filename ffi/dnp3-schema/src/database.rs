use class::ClassHandle;
use oo_bindgen::native_function::*;
use oo_bindgen::native_struct::StructElementType;
use oo_bindgen::*;

use crate::shared::SharedDefinitions;

pub fn define(
    lib: &mut LibraryBuilder,
    shared_def: &SharedDefinitions,
) -> Result<ClassHandle, BindingError> {
    let database = lib.declare_class("Database")?;

    let event_class = lib
        .define_native_enum("EventClass")?
        .push("None", "Does not generate events")?
        .push("Class1", "Class 1 event")?
        .push("Class2", "Class 2 event")?
        .push("Class3", "Class 3 event")?
        .doc("Event class")?
        .build()?;

    let event_mode = lib
        .define_native_enum("EventMode")?
        .push(
            "Detect",
            doc("Detect events in a type dependent fashion")
                .details("This is the default mode that should be used."),
        )?
        .push(
            "Force",
            "Produce an event whether the value has changed or not",
        )?
        .push("Suppress", "Never produce an event regardless of change")?
        .doc("Controls how events are processed when updating values in the database.")?
        .build()?;

    let update_options = lib.declare_native_struct("UpdateOptions")?;
    let update_options = lib
        .define_native_struct(&update_options)?
        .add(
            "update_static",
            StructElementType::Bool(Some(true)),
            "Optionnaly bypass updating the static database (the current value)",
        )?
        .add(
            "event_mode",
            StructElementType::Enum(event_mode, Some("Detect".to_string())),
            "Determines how/if an event is produced",
        )?
        .doc(
            doc("Options that control how the update is performed.")
                .details("99% of the time, the default value should be used."),
        )?
        .build()?;

    // Binary Input
    let binary_static_variation = lib
        .define_native_enum("StaticBinaryVariation")?
        .push("Group1Var1", "Binary input - packed format")?
        .push("Group1Var2", "Binary input - with flags")?
        .doc("Static binary input variation")?
        .build()?;

    let binary_event_variation = lib
        .define_native_enum("EventBinaryVariation")?
        .push("Group2Var1", "Binary input event - without time")?
        .push("Group2Var2", "Binary input event - with absolute time")?
        .push("Group2Var3", "Binary input event - with relative time")?
        .doc("Event binary input variation")?
        .build()?;

    let binary_config = lib.declare_native_struct("BinaryConfig")?;
    let binary_config = lib
        .define_native_struct(&binary_config)?
        .add(
            "static_variation",
            StructElementType::Enum(binary_static_variation, Some("Group1Var1".to_string())),
            "Default static variation",
        )?
        .add(
            "event_variation",
            StructElementType::Enum(binary_event_variation, Some("Group2Var1".to_string())),
            "Default event variation",
        )?
        .doc("Binary Input configuration")?
        .build()?;

    let binary_add_fn = lib
        .declare_native_function("database_add_binary")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point")?
        .param(
            "point_class",
            Type::Enum(event_class.clone()),
            "Event class",
        )?
        .param("config", Type::Struct(binary_config), "Configuration")?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully added, false otherwise",
        ))?
        .doc("Add a new Binary Input point")?
        .build()?;

    let binary_remove_fn = lib
        .declare_native_function("database_remove_binary")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point")?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully removed, false otherwise",
        ))?
        .doc("Remove a Binary Input point")?
        .build()?;

    let binary_update_fn = lib
        .declare_native_function("database_update_binary")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param(
            "value",
            Type::Struct(shared_def.binary_point.clone()),
            "New value of the point",
        )?
        .param(
            "options",
            Type::Struct(update_options.clone()),
            "Update options",
        )?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully updated, false otherwise",
        ))?
        .doc("Update a Binary Input point")?
        .build()?;

    let binary_get_fn = lib
        .declare_native_function("database_get_binary")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point to get")?
        .return_type(ReturnType::new(
            Type::Struct(shared_def.binary_point.clone()),
            "Binary Input point",
        ))?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Binary Input point")?
        .build()?;

    // Double-bit Binary Input
    let double_bit_binary_static_variation = lib
        .define_native_enum("StaticDoubleBitBinaryVariation")?
        .push("Group3Var1", "Double-bit binary input - packed format")?
        .push("Group3Var2", "Double-bit binary input - with flags")?
        .doc("Static double-bit binary input variation")?
        .build()?;

    let double_bit_binary_event_variation = lib
        .define_native_enum("EventDoubleBitBinaryVariation")?
        .push("Group4Var1", "Double-bit binary input event - without time")?
        .push(
            "Group4Var2",
            "Double-bit binary input event - with absolute time",
        )?
        .push(
            "Group4Var3",
            "Double-bit binary input event - with relative time",
        )?
        .doc("Event double-bit binary input variation")?
        .build()?;

    let double_bit_binary_config = lib.declare_native_struct("DoubleBitBinaryConfig")?;
    let double_bit_binary_config = lib
        .define_native_struct(&double_bit_binary_config)?
        .add(
            "static_variation",
            StructElementType::Enum(
                double_bit_binary_static_variation,
                Some("Group3Var1".to_string()),
            ),
            "Default static variation",
        )?
        .add(
            "event_variation",
            StructElementType::Enum(
                double_bit_binary_event_variation,
                Some("Group4Var1".to_string()),
            ),
            "Default event variation",
        )?
        .doc("Double-Bit Binary Input configuration")?
        .build()?;

    let double_bit_binary_add_fn = lib
        .declare_native_function("database_add_double_bit_binary")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point")?
        .param(
            "point_class",
            Type::Enum(event_class.clone()),
            "Event class",
        )?
        .param(
            "config",
            Type::Struct(double_bit_binary_config),
            "Configuration",
        )?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully added, false otherwise",
        ))?
        .doc("Add a new Double-Bit Binary Input point")?
        .build()?;

    let double_bit_binary_remove_fn = lib
        .declare_native_function("database_remove_double_bit_binary")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point")?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully removed, false otherwise",
        ))?
        .doc("Remove a Double-Bit Binary Input point")?
        .build()?;

    let double_bit_binary_update_fn = lib
        .declare_native_function("database_update_double_bit_binary")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param(
            "value",
            Type::Struct(shared_def.double_bit_binary_point.clone()),
            "New value of the point",
        )?
        .param(
            "options",
            Type::Struct(update_options.clone()),
            "Update options",
        )?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully updated, false otherwise",
        ))?
        .doc("Update a Double-Bit Binary Input point")?
        .build()?;

    let double_bit_binary_get_fn = lib
        .declare_native_function("database_get_double_bit_binary")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point to get")?
        .return_type(ReturnType::new(
            Type::Struct(shared_def.double_bit_binary_point.clone()),
            "Double-Bit Binary Input point",
        ))?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Double-Bit Binary Input point")?
        .build()?;

    // Binary Output Status
    let binary_output_status_static_variation = lib
        .define_native_enum("StaticBinaryOutputStatusVariation")?
        .push("Group10Var1", "Binary output - packed format")?
        .push("Group10Var2", "Binary output - output status with flags")?
        .doc("Static binary output status variation")?
        .build()?;

    let binary_output_status_event_variation = lib
        .define_native_enum("EventBinaryOutputStatusVariation")?
        .push("Group11Var1", "Binary output event - status without time")?
        .push("Group11Var2", "Binary output event - status with time")?
        .doc("Event binary output status variation")?
        .build()?;

    let binary_output_status_config = lib.declare_native_struct("BinaryOutputStatusConfig")?;
    let binary_output_status_config = lib
        .define_native_struct(&binary_output_status_config)?
        .add(
            "static_variation",
            StructElementType::Enum(
                binary_output_status_static_variation,
                Some("Group10Var1".to_string()),
            ),
            "Default static variation",
        )?
        .add(
            "event_variation",
            StructElementType::Enum(
                binary_output_status_event_variation,
                Some("Group11Var2".to_string()),
            ),
            "Default event variation",
        )?
        .doc("Binary Output Status configuration")?
        .build()?;

    let binary_output_status_add_fn = lib
        .declare_native_function("database_add_binary_output_status")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point")?
        .param(
            "point_class",
            Type::Enum(event_class.clone()),
            "Event class",
        )?
        .param(
            "config",
            Type::Struct(binary_output_status_config),
            "Configuration",
        )?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully added, false otherwise",
        ))?
        .doc("Add a new Binary Output Status point")?
        .build()?;

    let binary_output_status_remove_fn = lib
        .declare_native_function("database_remove_binary_output_status")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point")?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully removed, false otherwise",
        ))?
        .doc("Remove a Binary Output Status point")?
        .build()?;

    let binary_output_status_update_fn = lib
        .declare_native_function("database_update_binary_output_status")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param(
            "value",
            Type::Struct(shared_def.binary_output_status_point.clone()),
            "New value of the point",
        )?
        .param(
            "options",
            Type::Struct(update_options.clone()),
            "Update options",
        )?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully updated, false otherwise",
        ))?
        .doc("Update a Binary Output Status point")?
        .build()?;

    let binary_output_status_get_fn = lib
        .declare_native_function("database_get_binary_output_status")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point to get")?
        .return_type(ReturnType::new(
            Type::Struct(shared_def.binary_output_status_point.clone()),
            "Binary Output Status point",
        ))?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Binary Output Status point")?
        .build()?;

    // Counter
    let counter_static_variation = lib
        .define_native_enum("StaticCounterVariation")?
        .push("Group20Var1", "Counter - 32-bit with flag")?
        .push("Group20Var2", "Counter - 16-bit with flag")?
        .push("Group20Var5", "Counter - 32-bit without flag")?
        .push("Group20Var6", "Counter - 16-bit without flag")?
        .doc("Static counter variation")?
        .build()?;

    let counter_event_variation = lib
        .define_native_enum("EventCounterVariation")?
        .push("Group22Var1", "Counter event - 32-bit with flag")?
        .push("Group22Var2", "Counter event - 16-bit with flag")?
        .push("Group22Var5", "Counter event - 32-bit with flag and time")?
        .push("Group22Var6", "Counter event - 16-bit with flag and time")?
        .doc("Event counter variation")?
        .build()?;

    let counter_config = lib.declare_native_struct("CounterConfig")?;
    let counter_config = lib
        .define_native_struct(&counter_config)?
        .add(
            "static_variation",
            StructElementType::Enum(counter_static_variation, Some("Group20Var1".to_string())),
            "Default static variation",
        )?
        .add(
            "event_variation",
            StructElementType::Enum(counter_event_variation, Some("Group22Var1".to_string())),
            "Default event variation",
        )?
        .add(
            "deadband",
            StructElementType::Uint32(Some(0)),
            "Deadband value",
        )?
        .doc("Counter configuration")?
        .build()?;

    let counter_add_fn = lib
        .declare_native_function("database_add_counter")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point")?
        .param(
            "point_class",
            Type::Enum(event_class.clone()),
            "Event class",
        )?
        .param("config", Type::Struct(counter_config), "Configuration")?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully added, false otherwise",
        ))?
        .doc("Add a new Counter point")?
        .build()?;

    let counter_remove_fn = lib
        .declare_native_function("database_remove_counter")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point")?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully removed, false otherwise",
        ))?
        .doc("Remove a Counter point")?
        .build()?;

    let counter_update_fn = lib
        .declare_native_function("database_update_counter")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param(
            "value",
            Type::Struct(shared_def.counter_point.clone()),
            "New value of the point",
        )?
        .param(
            "options",
            Type::Struct(update_options.clone()),
            "Update options",
        )?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully updated, false otherwise",
        ))?
        .doc("Update a Counter point")?
        .build()?;

    let counter_get_fn = lib
        .declare_native_function("database_get_counter")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point to get")?
        .return_type(ReturnType::new(
            Type::Struct(shared_def.counter_point.clone()),
            "Counter point",
        ))?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Counter point")?
        .build()?;

    // Frozen Counter
    let frozen_counter_static_variation = lib
        .define_native_enum("StaticFrozenCounterVariation")?
        .push("Group21Var1", "Frozen Counter - 32-bit with flag")?
        .push("Group21Var2", "Frozen Counter - 16-bit with flag")?
        .push("Group21Var5", "Frozen Counter - 32-bit with flag and time")?
        .push("Group21Var6", "Frozen Counter - 16-bit with flag and time")?
        .push("Group21Var9", "Frozen Counter - 32-bit without flag")?
        .push("Group21Var10", "Frozen Counter - 16-bit without flag")?
        .doc("Static frozen counter variation")?
        .build()?;

    let frozen_counter_event_variation = lib
        .define_native_enum("EventFrozenCounterVariation")?
        .push("Group23Var1", "Frozen Counter event - 32-bit with flag")?
        .push("Group23Var2", "Frozen Counter event - 16-bit with flag")?
        .push(
            "Group23Var5",
            "Frozen Counter event - 32-bit with flag and time",
        )?
        .push(
            "Group23Var6",
            "Frozen Counter event - 16-bit with flag and time",
        )?
        .doc("Event frozen counter variation")?
        .build()?;

    let frozen_counter_config = lib.declare_native_struct("FrozenCounterConfig")?;
    let frozen_counter_config = lib
        .define_native_struct(&frozen_counter_config)?
        .add(
            "static_variation",
            StructElementType::Enum(
                frozen_counter_static_variation,
                Some("Group21Var1".to_string()),
            ),
            "Default static variation",
        )?
        .add(
            "event_variation",
            StructElementType::Enum(
                frozen_counter_event_variation,
                Some("Group23Var1".to_string()),
            ),
            "Default event variation",
        )?
        .add(
            "deadband",
            StructElementType::Uint32(Some(0)),
            "Deadband value",
        )?
        .doc("Frozen Counter configuration")?
        .build()?;

    let frozen_counter_add_fn = lib
        .declare_native_function("database_add_frozen_counter")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point")?
        .param(
            "point_class",
            Type::Enum(event_class.clone()),
            "Event class",
        )?
        .param(
            "config",
            Type::Struct(frozen_counter_config),
            "Configuration",
        )?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully added, false otherwise",
        ))?
        .doc("Add a new Frozen Counter point")?
        .build()?;

    let frozen_counter_remove_fn = lib
        .declare_native_function("database_remove_frozen_counter")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point")?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully removed, false otherwise",
        ))?
        .doc("Remove a Frozen Counter point")?
        .build()?;

    let frozen_counter_update_fn = lib
        .declare_native_function("database_update_frozen_counter")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param(
            "value",
            Type::Struct(shared_def.frozen_counter_point.clone()),
            "New value of the point",
        )?
        .param(
            "options",
            Type::Struct(update_options.clone()),
            "Update options",
        )?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully updated, false otherwise",
        ))?
        .doc("Update an Frozen Counter point")?
        .build()?;

    let frozen_counter_get_fn = lib
        .declare_native_function("database_get_frozen_counter")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point to get")?
        .return_type(ReturnType::new(
            Type::Struct(shared_def.frozen_counter_point.clone()),
            "Frozen Counter point",
        ))?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Frozen Counter point")?
        .build()?;

    // Analog
    let analog_static_variation = lib
        .define_native_enum("StaticAnalogVariation")?
        .push("Group30Var1", "Analog input - 32-bit with flag")?
        .push("Group30Var2", "Analog input - 16-bit with flag")?
        .push("Group30Var3", "Analog input - 32-bit without flag")?
        .push("Group30Var4", "Analog input - 16-bit without flag")?
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
        .define_native_enum("EventAnalogVariation")?
        .push("Group32Var1", "Analog input event - 32-bit without time")?
        .push("Group32Var2", "Analog input event - 16-bit without time")?
        .push("Group32Var3", "Analog input event - 32-bit with time")?
        .push("Group32Var4", "Analog input event - 16-bit with time")?
        .push(
            "Group32Var5",
            "Analog input event - single-precision, floating-point without time",
        )?
        .push(
            "Group32Var6",
            "Analog input event - double-precision, floating-point without time",
        )?
        .push(
            "Group32Var7",
            "Analog input event - single-precision, floating-point with time",
        )?
        .push(
            "Group32Var8",
            "Analog input event - double-precision, floating-point with time",
        )?
        .doc("Event analog variation")?
        .build()?;

    let analog_config = lib.declare_native_struct("AnalogConfig")?;
    let analog_config = lib
        .define_native_struct(&analog_config)?
        .add(
            "static_variation",
            StructElementType::Enum(analog_static_variation, Some("Group30Var1".to_string())),
            "Default static variation",
        )?
        .add(
            "event_variation",
            StructElementType::Enum(analog_event_variation, Some("Group32Var1".to_string())),
            "Default event variation",
        )?
        .add(
            "deadband",
            StructElementType::Double(Some(0.0)),
            "Deadband value",
        )?
        .doc("Analog configuration")?
        .build()?;

    let analog_add_fn = lib
        .declare_native_function("database_add_analog")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point")?
        .param(
            "point_class",
            Type::Enum(event_class.clone()),
            "Event class",
        )?
        .param("config", Type::Struct(analog_config), "Configuration")?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully added, false otherwise",
        ))?
        .doc("Add a new Analog point")?
        .build()?;

    let analog_remove_fn = lib
        .declare_native_function("database_remove_analog")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point")?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully removed, false otherwise",
        ))?
        .doc("Remove an Analog point")?
        .build()?;

    let analog_update_fn = lib
        .declare_native_function("database_update_analog")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param(
            "value",
            Type::Struct(shared_def.analog_point.clone()),
            "New value of the point",
        )?
        .param(
            "options",
            Type::Struct(update_options.clone()),
            "Update options",
        )?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully updated, false otherwise",
        ))?
        .doc("Update a Analog point")?
        .build()?;

    let analog_get_fn = lib
        .declare_native_function("database_get_analog")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point to get")?
        .return_type(ReturnType::new(
            Type::Struct(shared_def.analog_point.clone()),
            "Analog point",
        ))?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Analog point")?
        .build()?;

    // Analog Output Status
    let analog_output_status_static_variation = lib
        .define_native_enum("StaticAnalogOutputStatusVariation")?
        .push("Group40Var1", "Analog output status - 32-bit with flag")?
        .push("Group40Var2", "Analog output status - 16-bit with flag")?
        .push(
            "Group40Var3",
            "Analog output status - single-precision, floating-point with flag",
        )?
        .push(
            "Group40Var4",
            "Analog output status - double-precision, floating-point with flag",
        )?
        .doc("Static analog output status variation")?
        .build()?;

    let analog_output_status_event_variation = lib
        .define_native_enum("EventAnalogOutputStatusVariation")?
        .push("Group42Var1", "Analog output event - 32-bit without time")?
        .push("Group42Var2", "Analog output event - 16-bit without time")?
        .push("Group42Var3", "Analog output event - 32-bit with time")?
        .push("Group42Var4", "Analog output event - 16-bit with time")?
        .push(
            "Group42Var5",
            "Analog output event - single-precision, floating-point without time",
        )?
        .push(
            "Group42Var6",
            "Analog output event - double-precision, floating-point without time",
        )?
        .push(
            "Group42Var7",
            "Analog output event - single-precision, floating-point with time",
        )?
        .push(
            "Group42Var8",
            "Analog output event - double-precision, floating-point with time",
        )?
        .doc("Event analog output status variation")?
        .build()?;

    let analog_output_status_config = lib.declare_native_struct("AnalogOutputStatusConfig")?;
    let analog_output_status_config = lib
        .define_native_struct(&analog_output_status_config)?
        .add(
            "static_variation",
            StructElementType::Enum(
                analog_output_status_static_variation,
                Some("Group40Var1".to_string()),
            ),
            "Default static variation",
        )?
        .add(
            "event_variation",
            StructElementType::Enum(
                analog_output_status_event_variation,
                Some("Group42Var1".to_string()),
            ),
            "Default event variation",
        )?
        .add(
            "deadband",
            StructElementType::Double(Some(0.0)),
            "Deadband value",
        )?
        .doc("Analog Output Status configuration")?
        .build()?;

    let analog_output_status_add_fn = lib
        .declare_native_function("database_add_analog_output_status")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point")?
        .param(
            "point_class",
            Type::Enum(event_class.clone()),
            "Event class",
        )?
        .param(
            "config",
            Type::Struct(analog_output_status_config),
            "Configuration",
        )?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully added, false otherwise",
        ))?
        .doc("Add a new Analog Output Status point")?
        .build()?;

    let analog_output_status_remove_fn = lib
        .declare_native_function("database_remove_analog_output_status")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point")?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully removed, false otherwise",
        ))?
        .doc("Remove an Analog Output Status point")?
        .build()?;

    let analog_output_status_update_fn = lib
        .declare_native_function("database_update_analog_output_status")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param(
            "value",
            Type::Struct(shared_def.analog_output_status_point.clone()),
            "New value of the point",
        )?
        .param(
            "options",
            Type::Struct(update_options.clone()),
            "Update options",
        )?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully updated, false otherwise",
        ))?
        .doc("Update a Analog Output Status point")?
        .build()?;

    let analog_output_status_get_fn = lib
        .declare_native_function("database_get_analog_output_status")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point to get")?
        .return_type(ReturnType::new(
            Type::Struct(shared_def.analog_output_status_point.clone()),
            "Analog Output Status point",
        ))?
        .fails_with(shared_def.error_type.clone())?
        .doc("Get a Analog Output Status point")?
        .build()?;

    // Octet String
    let octet_string_class = lib.declare_class("OctetStringValue")?;

    let octet_string_new_fn = lib
        .declare_native_function("octet_string_new")?
        .return_type(ReturnType::new(
            Type::ClassRef(octet_string_class.clone()),
            "Empty octet string",
        ))?
        .doc("Create a new octet string")?
        .build()?;

    let octet_string_destroy_fn = lib
        .declare_native_function("octet_string_destroy")?
        .param(
            "octet_string",
            Type::ClassRef(octet_string_class.clone()),
            "Octet String to destroy",
        )?
        .return_type(ReturnType::void())?
        .doc("Deallocate an octet string")?
        .build()?;

    let octet_string_add_fn = lib
        .declare_native_function("octet_string_add")?
        .param(
            "octet_string",
            Type::ClassRef(octet_string_class),
            "Octet String to modify",
        )?
        .param("value", Type::Uint8, "Byte to add")?
        .return_type(ReturnType::void())?
        .doc("Create a new octet string")?
        .build()?;

    let octet_string_collection = lib.define_collection(
        &octet_string_new_fn,
        &octet_string_destroy_fn,
        &octet_string_add_fn,
    )?;

    let octet_string_add_fn = lib
        .declare_native_function("database_add_octet_string")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point")?
        .param("point_class", Type::Enum(event_class), "Event class")?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully added, false otherwise",
        ))?
        .doc("Add a new Octet String point")?
        .build()?;

    let octet_string_remove_fn = lib
        .declare_native_function("database_remove_octet_string")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the point")?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully removed, false otherwise",
        ))?
        .doc("Remove an Octet String point")?
        .build()?;

    let octet_string_update_fn = lib
        .declare_native_function("database_update_octet_string")?
        .param("db", Type::ClassRef(database.clone()), "Database")?
        .param("index", Type::Uint16, "Index of the octet string")?
        .param(
            "value",
            Type::Collection(octet_string_collection),
            "New value of the point",
        )?
        .param("options", Type::Struct(update_options), "Update options")?
        .return_type(ReturnType::new(
            Type::Bool,
            "True if the point was successfully updated, false otherwise",
        ))?
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
