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

use oo_bindgen::model::{BackTraced, EnumHandle, LibraryBuilder};

pub(crate) fn define(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let variation = lib
        .define_enum("variation")?
        .push("group0", "Device Attributes - Specific attribute (variations 1 to 253 and 255). Only reported in {struct:header_info}: it can't name the attribute, so a request that uses it fails with {enum:read_error.bad_encoding}. Read a specific attribute with {class:request.add_specific_attribute()} and all of them with {enum:variation.group0_var254}")?
        .push("group0_var254", "Device Attributes - All attributes request")?
        .push("group1_var0", "Binary Input - Any variation")?
        .push("group1_var1", "Binary Input - Packed format")?
        .push("group1_var2", "Binary Input - With flags")?
        .push("group2_var0", "Binary Input Event - Any variation")?
        .push("group2_var1", "Binary Input Event - Without time")?
        .push("group2_var2", "Binary Input Event - With absolute time")?
        .push("group2_var3", "Binary Input Event - With relative time")?
        .push("group3_var0", "Double-bit Binary Input - Any variation")?
        .push("group3_var1", "Double-bit Binary Input - Packed format")?
        .push("group3_var2", "Double-bit Binary Input - With flags")?
        .push("group4_var0", "Double-bit Binary Input Event - Any variation")?
        .push("group4_var1", "Double-bit Binary Input Event - Without time")?
        .push("group4_var2", "Double-bit Binary Input Event - With absolute time")?
        .push("group4_var3", "Double-bit Binary Input Event - With relative time")?
        .push("group10_var0", "Binary Output - Any variation")?
        .push("group10_var1", "Binary Output - Packed format")?
        .push("group10_var2", "Binary Output - With flags")?
        .push("group11_var0", "Binary Output Event - Any variation")?
        .push("group11_var1", "Binary Output Event - Without time")?
        .push("group11_var2", "Binary Output Event - With time")?
        .push("group12_var1", "Binary Output Command - Control relay output block")?
        .push("group13_var1", "Binary Output Command Event - Without time")?
        .push("group13_var2", "Binary Output Command Event - With time")?
        .push("group20_var0", "Counter - Any variation")?
        .push("group20_var1", "Counter - 32-bit with flags")?
        .push("group20_var2", "Counter - 16-bit with flags")?
        .push("group20_var5", "Counter - 32-bit without flags")?
        .push("group20_var6", "Counter - 16-bit without flags")?
        .push("group21_var0", "Frozen Counter - Any variation")?
        .push("group21_var1", "Frozen Counter - 32-bit with flags")?
        .push("group21_var2", "Frozen Counter - 16-bit with flags")?
        .push("group21_var5", "Frozen Counter - 32-bit with flags and time")?
        .push("group21_var6", "Frozen Counter - 16-bit with flags and time")?
        .push("group21_var9", "Frozen Counter - 32-bit without flags")?
        .push("group21_var10", "Frozen Counter - 16-bit without flags")?
        .push("group22_var0", "Counter Event - Any variation")?
        .push("group22_var1", "Counter Event - 32-bit without time")?
        .push("group22_var2", "Counter Event - 16-bit without time")?
        .push("group22_var5", "Counter Event - 32-bit with time")?
        .push("group22_var6", "Counter Event - 16-bit with time")?
        .push("group23_var0", "Frozen Counter Event - Any variation")?
        .push("group23_var1", "Frozen Counter Event - 32-bit without time")?
        .push("group23_var2", "Frozen Counter Event - 16-bit without time")?
        .push("group23_var5", "Frozen Counter Event - 32-bit with time")?
        .push("group23_var6", "Frozen Counter Event - 16-bit with time")?
        .push("group30_var0", "Analog Input - Any variation")?
        .push("group30_var1", "Analog Input - 32-bit with flags")?
        .push("group30_var2", "Analog Input - 16-bit with flags")?
        .push("group30_var3", "Analog Input - 32-bit without flags")?
        .push("group30_var4", "Analog Input - 16-bit without flags")?
        .push("group30_var5", "Analog Input - Single-precision floating point with flags")?
        .push("group30_var6", "Analog Input - Double-precision floating point with flags")?
        .push("group31_var0", "Frozen Analog Input - Any variation")?
        .push("group31_var1", "Frozen Analog Input - 32-bit with flags")?
        .push("group31_var2", "Frozen Analog Input - 16-bit with flags")?
        .push("group31_var3", "Frozen Analog Input - 32-bit with flags and time of freeze")?
        .push("group31_var4", "Frozen Analog Input - 16-bit with flags and time of freeze")?
        .push("group31_var5", "Frozen Analog Input - 32-bit without flags")?
        .push("group31_var6", "Frozen Analog Input - 16-bit without flags")?
        .push("group31_var7", "Frozen Analog Input - Single-precision floating point with flags")?
        .push("group31_var8", "Frozen Analog Input - Double-precision floating point with flags")?
        .push("group32_var0", "Analog Input Event - Any variation")?
        .push("group32_var1", "Analog Input Event - 32-bit without time")?
        .push("group32_var2", "Analog Input Event - 16-bit without time")?
        .push("group32_var3", "Analog Input Event - 32-bit with time")?
        .push("group32_var4", "Analog Input Event - 16-bit with time")?
        .push("group32_var5", "Analog Input Event - Single-precision floating point without time")?
        .push("group32_var6", "Analog Input Event - Double-precision floating point without time")?
        .push("group32_var7", "Analog Input Event - Single-precision floating point with time")?
        .push("group32_var8", "Analog Input Event - Double-precision floating point with time")?
        .push("group33_var0", "Frozen Analog Input Event - Any variation")?
        .push("group33_var1", "Frozen Analog Input Event - 32-bit without time")?
        .push("group33_var2", "Frozen Analog Input Event - 16-bit without time")?
        .push("group33_var3", "Frozen Analog Input Event - 32-bit with time of freeze")?
        .push("group33_var4", "Frozen Analog Input Event - 16-bit with time of freeze")?
        .push("group33_var5", "Frozen Analog Input Event - Single-precision floating point without time")?
        .push("group33_var6", "Frozen Analog Input Event - Double-precision floating point without time")?
        .push("group33_var7", "Frozen Analog Input Event - Single-precision floating point with time of freeze")?
        .push("group33_var8", "Frozen Analog Input Event - Double-precision floating point with time of freeze")?
        .push("group34_var0", "Analog Input Reporting Deadband - Any variation")?
        .push("group34_var1", "Analog Input Reporting Deadband - 16-bit")?
        .push("group34_var2", "Analog Input Reporting Deadband - 32-bit")?
        .push("group34_var3", "Analog Input Reporting Deadband - Single-precision floating point")?
        .push("group40_var0", "Analog Output Status - Any variation")?
        .push("group40_var1", "Analog Output Status - 32-bit with flags")?
        .push("group40_var2", "Analog Output Status - 16-bit with flags")?
        .push("group40_var3", "Analog Output Status - Single-precision floating point with flags")?
        .push("group40_var4", "Analog Output Status - Double-precision floating point with flags")?
        .push("group41_var1", "Analog Output - 32-bit")?
        .push("group41_var2", "Analog Output - 16-bit")?
        .push("group41_var3", "Analog Output - Single-precision floating point")?
        .push("group41_var4", "Analog Output - Double-precision floating point")?
        .push("group42_var0", "Analog Output Event - Any variation")?
        .push("group42_var1", "Analog Output Event - 32-bit without time")?
        .push("group42_var2", "Analog Output Event - 16-bit without time")?
        .push("group42_var3", "Analog Output Event - 32-bit with time")?
        .push("group42_var4", "Analog Output Event - 16-bit with time")?
        .push("group42_var5", "Analog Output Event - Single-precision floating point without time")?
        .push("group42_var6", "Analog Output Event - Double-precision floating point without time")?
        .push("group42_var7", "Analog Output Event - Single-precision floating point with time")?
        .push("group42_var8", "Analog Output Event - Double-precision floating point with time")?
        .push("group43_var1", "Analog Output Command Event - 32-bit without time")?
        .push("group43_var2", "Analog Output Command Event - 16-bit without time")?
        .push("group43_var3", "Analog Output Command Event - 32-bit with time")?
        .push("group43_var4", "Analog Output Command Event - 16-bit with time")?
        .push("group43_var5", "Analog Output Command Event - Single-precision floating point without time")?
        .push("group43_var6", "Analog Output Command Event - Double-precision floating point without time")?
        .push("group43_var7", "Analog Output Command Event - Single-precision floating point with time")?
        .push("group43_var8", "Analog Output Command Event - Double-precision floating point with time")?
        .push("group50_var1", "Time and Date - Absolute time")?
        .push("group50_var2", "Time and Date - Absolute time and interval")?
        .push("group50_var3", "Time and Date - Last recorded time")?
        .push("group50_var4", "Time and Date - Indexed time and long interval")?
        .push("group51_var1", "Time and Date CTO - Synchronized absolute time")?
        .push("group51_var2", "Time and Date CTO - Unsynchronized absolute time")?
        .push("group52_var1", "Time Delay - Coarse (seconds)")?
        .push("group52_var2", "Time Delay - Fine (milliseconds)")?
        .push("group60_var1", "Class Data - Class 0")?
        .push("group60_var2", "Class Data - Class 1")?
        .push("group60_var3", "Class Data - Class 2")?
        .push("group60_var4", "Class Data - Class 3")?
        .push("group70_var2", "File Control - Authentication")?
        .push("group70_var3", "File Control - File command")?
        .push("group70_var4", "File Control - File command status")?
        .push("group70_var5", "File Control - File transport")?
        .push("group70_var6", "File Control - File transport status")?
        .push("group70_var7", "File Control - File descriptor")?
        .push("group70_var8", "File Control - File specification string")?
        .push("group80_var1", "Internal Indications - Packed format")?
        .push("group102_var0", "Unsigned Integer - Any variation")?
        .push("group102_var1", "Unsigned Integer - 8-bit")?
        .push("group110", "Octet String - Sized by variation")?
        .push("group111", "Octet String Event - Sized by variation")?
        .doc("Group/Variation")?
        .build()?;
    Ok(variation)
}
