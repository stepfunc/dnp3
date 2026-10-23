// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//  _   _         ______    _ _ _   _             _ _ _
// | \ | |       |  ____|  | (_) | (_)           | | | |
// |  \| | ___   | |__   __| |_| |_ _ _ __   __ _| | | |
// | . ` |/ _ \  |  __| / _` | | __| | '_ \ / _` | | | |
// | |\  | (_) | | |___| (_| | | |_| | | | | (_| |_|_|_|
// |_| \_|\___/  |______\__,_|_|\__|_|_| |_|\__, (_|_|_)
//                                           __/ |
//                                          |___/
//
// This file is auto-generated. Do not edit manually
//

use oo_bindgen::model::{doc, BackTraced, EnumHandle, LibraryBuilder};

/// Defines the static and event variation enums of the binary input points
pub(crate) fn define_binary_input_variations(
    lib: &mut LibraryBuilder,
) -> BackTraced<(EnumHandle, EnumHandle)> {
    let static_variation = lib
        .define_enum("static_binary_input_variation")?
        .push("group1_var1", "Binary Input - Packed format")?
        .push("group1_var2", "Binary Input - With flags")?
        .doc("Static binary input variation")?
        .build()?;

    let event_variation = lib
        .define_enum("event_binary_input_variation")?
        .push("group2_var1", "Binary Input Event - Without time")?
        .push(
            "group2_var2",
            doc("Binary Input Event - With absolute time")
                .details("Events recorded with unsynchronized time are reported as {enum:event_binary_input_variation.group2_var3} after an unsynchronized common time of occurrence (g51v2), and events recorded without time as {enum:event_binary_input_variation.group2_var1}."),
        )?
        .push(
            "group2_var3",
            doc("Binary Input Event - With relative time")
                .details("Events recorded without time are reported as {enum:event_binary_input_variation.group2_var1}."),
        )?
        .doc("Event binary input variation")?
        .build()?;

    Ok((static_variation, event_variation))
}

/// Defines the static and event variation enums of the double-bit binary input points
pub(crate) fn define_double_bit_binary_input_variations(
    lib: &mut LibraryBuilder,
) -> BackTraced<(EnumHandle, EnumHandle)> {
    let static_variation = lib
        .define_enum("static_double_bit_binary_input_variation")?
        .push("group3_var1", "Double-bit Binary Input - Packed format")?
        .push("group3_var2", "Double-bit Binary Input - With flags")?
        .doc("Static double-bit binary input variation")?
        .build()?;

    let event_variation = lib
        .define_enum("event_double_bit_binary_input_variation")?
        .push(
            "group4_var1",
            "Double-bit Binary Input Event - Without time",
        )?
        .push(
            "group4_var2",
            doc("Double-bit Binary Input Event - With absolute time")
                .details("Events recorded with unsynchronized time are reported as {enum:event_double_bit_binary_input_variation.group4_var3} after an unsynchronized common time of occurrence (g51v2), and events recorded without time as {enum:event_double_bit_binary_input_variation.group4_var1}."),
        )?
        .push(
            "group4_var3",
            doc("Double-bit Binary Input Event - With relative time")
                .details("Events recorded without time are reported as {enum:event_double_bit_binary_input_variation.group4_var1}."),
        )?
        .doc("Event double-bit binary input variation")?
        .build()?;

    Ok((static_variation, event_variation))
}

/// Defines the static and event variation enums of the binary output status points
pub(crate) fn define_binary_output_status_variations(
    lib: &mut LibraryBuilder,
) -> BackTraced<(EnumHandle, EnumHandle)> {
    let static_variation = lib
        .define_enum("static_binary_output_status_variation")?
        .push("group10_var1", "Binary Output - Packed format")?
        .push("group10_var2", "Binary Output - With flags")?
        .doc("Static binary output status variation")?
        .build()?;

    let event_variation = lib
        .define_enum("event_binary_output_status_variation")?
        .push("group11_var1", "Binary Output Event - Without time")?
        .push(
            "group11_var2",
            doc("Binary Output Event - With time")
                .details("Events recorded without time are reported as {enum:event_binary_output_status_variation.group11_var1}."),
        )?
        .doc("Event binary output status variation")?
        .build()?;

    Ok((static_variation, event_variation))
}

/// Defines the static and event variation enums of the counter points
pub(crate) fn define_counter_variations(
    lib: &mut LibraryBuilder,
) -> BackTraced<(EnumHandle, EnumHandle)> {
    let static_variation = lib
        .define_enum("static_counter_variation")?
        .push("group20_var1", "Counter - 32-bit with flags")?
        .push("group20_var2", "Counter - 16-bit with flags")?
        .push("group20_var5", "Counter - 32-bit without flags")?
        .push("group20_var6", "Counter - 16-bit without flags")?
        .doc("Static counter variation")?
        .build()?;

    let event_variation = lib
        .define_enum("event_counter_variation")?
        .push("group22_var1", "Counter Event - 32-bit without time")?
        .push("group22_var2", "Counter Event - 16-bit without time")?
        .push(
            "group22_var5",
            doc("Counter Event - 32-bit with time")
                .details("Events recorded without time are reported as {enum:event_counter_variation.group22_var1}."),
        )?
        .push(
            "group22_var6",
            doc("Counter Event - 16-bit with time")
                .details("Events recorded without time are reported as {enum:event_counter_variation.group22_var2}."),
        )?
        .doc("Event counter variation")?
        .build()?;

    Ok((static_variation, event_variation))
}

/// Defines the static and event variation enums of the frozen counter points
pub(crate) fn define_frozen_counter_variations(
    lib: &mut LibraryBuilder,
) -> BackTraced<(EnumHandle, EnumHandle)> {
    let static_variation = lib
        .define_enum("static_frozen_counter_variation")?
        .push("group21_var1", "Frozen Counter - 32-bit with flags")?
        .push("group21_var2", "Frozen Counter - 16-bit with flags")?
        .push(
            "group21_var5",
            "Frozen Counter - 32-bit with flags and time",
        )?
        .push(
            "group21_var6",
            "Frozen Counter - 16-bit with flags and time",
        )?
        .push("group21_var9", "Frozen Counter - 32-bit without flags")?
        .push("group21_var10", "Frozen Counter - 16-bit without flags")?
        .doc("Static frozen counter variation")?
        .build()?;

    let event_variation = lib
        .define_enum("event_frozen_counter_variation")?
        .push("group23_var1", "Frozen Counter Event - 32-bit without time")?
        .push("group23_var2", "Frozen Counter Event - 16-bit without time")?
        .push(
            "group23_var5",
            doc("Frozen Counter Event - 32-bit with time")
                .details("Events recorded without time are reported as {enum:event_frozen_counter_variation.group23_var1}."),
        )?
        .push(
            "group23_var6",
            doc("Frozen Counter Event - 16-bit with time")
                .details("Events recorded without time are reported as {enum:event_frozen_counter_variation.group23_var2}."),
        )?
        .doc("Event frozen counter variation")?
        .build()?;

    Ok((static_variation, event_variation))
}

/// Defines the static and event variation enums of the analog input points
pub(crate) fn define_analog_input_variations(
    lib: &mut LibraryBuilder,
) -> BackTraced<(EnumHandle, EnumHandle)> {
    let static_variation = lib
        .define_enum("static_analog_input_variation")?
        .push("group30_var1", "Analog Input - 32-bit with flags")?
        .push("group30_var2", "Analog Input - 16-bit with flags")?
        .push("group30_var3", "Analog Input - 32-bit without flags")?
        .push("group30_var4", "Analog Input - 16-bit without flags")?
        .push(
            "group30_var5",
            "Analog Input - Single-precision floating point with flags",
        )?
        .push(
            "group30_var6",
            "Analog Input - Double-precision floating point with flags",
        )?
        .doc("Static analog input variation")?
        .build()?;

    let event_variation = lib
        .define_enum("event_analog_input_variation")?
        .push("group32_var1", "Analog Input Event - 32-bit without time")?
        .push("group32_var2", "Analog Input Event - 16-bit without time")?
        .push(
            "group32_var3",
            doc("Analog Input Event - 32-bit with time")
                .details("Events recorded without time are reported as {enum:event_analog_input_variation.group32_var1}."),
        )?
        .push(
            "group32_var4",
            doc("Analog Input Event - 16-bit with time")
                .details("Events recorded without time are reported as {enum:event_analog_input_variation.group32_var2}."),
        )?
        .push(
            "group32_var5",
            "Analog Input Event - Single-precision floating point without time",
        )?
        .push(
            "group32_var6",
            "Analog Input Event - Double-precision floating point without time",
        )?
        .push(
            "group32_var7",
            doc("Analog Input Event - Single-precision floating point with time")
                .details("Events recorded without time are reported as {enum:event_analog_input_variation.group32_var5}."),
        )?
        .push(
            "group32_var8",
            doc("Analog Input Event - Double-precision floating point with time")
                .details("Events recorded without time are reported as {enum:event_analog_input_variation.group32_var6}."),
        )?
        .doc("Event analog input variation")?
        .build()?;

    Ok((static_variation, event_variation))
}

/// Defines the static and event variation enums of the analog output status points
pub(crate) fn define_analog_output_status_variations(
    lib: &mut LibraryBuilder,
) -> BackTraced<(EnumHandle, EnumHandle)> {
    let static_variation = lib
        .define_enum("static_analog_output_status_variation")?
        .push("group40_var1", "Analog Output Status - 32-bit with flags")?
        .push("group40_var2", "Analog Output Status - 16-bit with flags")?
        .push(
            "group40_var3",
            "Analog Output Status - Single-precision floating point with flags",
        )?
        .push(
            "group40_var4",
            "Analog Output Status - Double-precision floating point with flags",
        )?
        .doc("Static analog output status variation")?
        .build()?;

    let event_variation = lib
        .define_enum("event_analog_output_status_variation")?
        .push("group42_var1", "Analog Output Event - 32-bit without time")?
        .push("group42_var2", "Analog Output Event - 16-bit without time")?
        .push(
            "group42_var3",
            doc("Analog Output Event - 32-bit with time")
                .details("Events recorded without time are reported as {enum:event_analog_output_status_variation.group42_var1}."),
        )?
        .push(
            "group42_var4",
            doc("Analog Output Event - 16-bit with time")
                .details("Events recorded without time are reported as {enum:event_analog_output_status_variation.group42_var2}."),
        )?
        .push(
            "group42_var5",
            "Analog Output Event - Single-precision floating point without time",
        )?
        .push(
            "group42_var6",
            "Analog Output Event - Double-precision floating point without time",
        )?
        .push(
            "group42_var7",
            doc("Analog Output Event - Single-precision floating point with time")
                .details("Events recorded without time are reported as {enum:event_analog_output_status_variation.group42_var5}."),
        )?
        .push(
            "group42_var8",
            doc("Analog Output Event - Double-precision floating point with time")
                .details("Events recorded without time are reported as {enum:event_analog_output_status_variation.group42_var6}."),
        )?
        .doc("Event analog output status variation")?
        .build()?;

    Ok((static_variation, event_variation))
}
