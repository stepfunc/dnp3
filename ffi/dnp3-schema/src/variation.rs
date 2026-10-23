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
        .push("group0", "Device Attributes - Variations 0 to 253 and 255")?
        .push("group0_var254", "Device Attributes - Non-Specific All Attributes Request")?
        .push("group1_var0", "Binary Input - Default variation")?
        .push("group1_var1", "Binary Input - Packed format")?
        .push("group1_var2", "Binary Input - With flags")?
        .push("group2_var0", "Binary Input Event - Default variation")?
        .push("group2_var1", "Binary Input Event - Without time")?
        .push("group2_var2", "Binary Input Event - With absolute time")?
        .push("group2_var3", "Binary Input Event - With relative time")?
        .push("group3_var0", "Double-bit Binary Input - Default variation")?
        .push("group3_var1", "Double-bit Binary Input - Packed format")?
        .push("group3_var2", "Double-bit Binary Input - With flags")?
        .push("group4_var0", "Double-bit Binary Input Event - Default variation")?
        .push("group4_var1", "Double-bit Binary Input Event - Without time")?
        .push("group4_var2", "Double-bit Binary Input Event - With absolute time")?
        .push("group4_var3", "Double-bit Binary Input Event - With relative time")?
        .push("group10_var0", "Binary Output - Default variation")?
        .push("group10_var1", "Binary Output - Packed format")?
        .push("group10_var2", "Binary Output - With flags")?
        .push("group11_var0", "Binary Output Event - Default variation")?
        .push("group11_var1", "Binary Output Event - Without time")?
        .push("group11_var2", "Binary Output Event - With time")?
        .push("group12_var1", "Binary Output Command - Control Relay Output Block")?
        .push("group13_var1", "Binary Output Command Event - command status without time")?
        .push("group13_var2", "Binary Output Command Event - command status with time")?
        .push("group20_var0", "Counter - Default variation")?
        .push("group20_var1", "Counter - 32-bit with flags")?
        .push("group20_var2", "Counter - 16-bit with flags")?
        .push("group20_var5", "Counter - 32-bit without flag")?
        .push("group20_var6", "Counter - 16-bit without flag")?
        .push("group21_var0", "Frozen Counter - Default variation")?
        .push("group21_var1", "Frozen Counter - 32-bit with flags")?
        .push("group21_var2", "Frozen Counter - 16-bit with flags")?
        .push("group21_var5", "Frozen Counter - 32-bit with flags and time")?
        .push("group21_var6", "Frozen Counter - 16-bit with flags and time")?
        .push("group21_var9", "Frozen Counter - 32-bit without flag")?
        .push("group21_var10", "Frozen Counter - 16-bit without flag")?
        .push("group22_var0", "Counter Event - Default variation")?
        .push("group22_var1", "Counter Event - 32-bit with flags")?
        .push("group22_var2", "Counter Event - 16-bit with flags")?
        .push("group22_var5", "Counter Event - 32-bit with flags and time")?
        .push("group22_var6", "Counter Event - 16-bit with flags and time")?
        .push("group23_var0", "Frozen Counter Event - Default variation")?
        .push("group23_var1", "Frozen Counter Event - 32-bit with flags")?
        .push("group23_var2", "Frozen Counter Event - 16-bit with flags")?
        .push("group23_var5", "Frozen Counter Event - 32-bit with flags and time")?
        .push("group23_var6", "Frozen Counter Event - 16-bit with flags and time")?
        .push("group30_var0", "Analog Input - Default variation")?
        .push("group30_var1", "Analog Input - 32-bit with flags")?
        .push("group30_var2", "Analog Input - 16-bit with flags")?
        .push("group30_var3", "Analog Input - 32-bit without flag")?
        .push("group30_var4", "Analog Input - 16-bit without flag")?
        .push("group30_var5", "Analog Input - Single-precision floating point with flags")?
        .push("group30_var6", "Analog Input - Double-precision floating point with flags")?
        .push("group31_var0", "Frozen Analog Input - Default variation")?
        .push("group31_var1", "Frozen Analog Input - 32-bit with flags")?
        .push("group31_var2", "Frozen Analog Input - 16-bit with flags")?
        .push("group31_var3", "Frozen Analog Input - 32-bit with flags and time-of-freeze")?
        .push("group31_var4", "Frozen Analog Input - 16-bit with flags and time-of-freeze")?
        .push("group31_var5", "Frozen Analog Input - 32-bit without flags")?
        .push("group31_var6", "Frozen Analog Input - 16-bit without flags")?
        .push("group31_var7", "Frozen Analog Input - Single-precision floating point with flags")?
        .push("group31_var8", "Frozen Analog Input - Double-precision floating point with flags")?
        .push("group32_var0", "Analog Input Event - Default variation")?
        .push("group32_var1", "Analog Input Event - 32-bit without time")?
        .push("group32_var2", "Analog Input Event - 16-bit without time")?
        .push("group32_var3", "Analog Input Event - 32-bit with time")?
        .push("group32_var4", "Analog Input Event - 16-bit with time")?
        .push("group32_var5", "Analog Input Event - Single-precision floating point without time")?
        .push("group32_var6", "Analog Input Event - Double-precision floating point without time")?
        .push("group32_var7", "Analog Input Event - Single-precision floating point with time")?
        .push("group32_var8", "Analog Input Event - Double-precision floating point with time")?
        .push("group33_var0", "Frozen Analog Input Event - Default variation")?
        .push("group33_var1", "Frozen Analog Input Event - 32-bit without time")?
        .push("group33_var2", "Frozen Analog Input Event - 16-bit without time")?
        .push("group33_var3", "Frozen Analog Input Event - 32-bit with time")?
        .push("group33_var4", "Frozen Analog Input Event - 16-bit with time")?
        .push("group33_var5", "Frozen Analog Input Event - Single-precision floating point without time")?
        .push("group33_var6", "Frozen Analog Input Event - Double-precision floating point without time")?
        .push("group33_var7", "Frozen Analog Input Event - Single-precision floating point with time")?
        .push("group33_var8", "Frozen Analog Input Event - Double-precision floating point with time")?
        .push("group34_var0", "Analog Input Reporting Deadband - Default variation")?
        .push("group34_var1", "Analog Input Reporting Deadband - 16-bit")?
        .push("group34_var2", "Analog Input Reporting Deadband - 32-bit")?
        .push("group34_var3", "Analog Input Reporting Deadband - Single-precision floating point")?
        .push("group40_var0", "Analog Output Status - Default variation")?
        .push("group40_var1", "Analog Output Status - 32-bit with flags")?
        .push("group40_var2", "Analog Output Status - 16-bit with flags")?
        .push("group40_var3", "Analog Output Status - Single-precision floating point with flags")?
        .push("group40_var4", "Analog Output Status - Double-precision floating point with flags")?
        .push("group41_var1", "Analog Output - 32-bit")?
        .push("group41_var2", "Analog Output - 16-bit")?
        .push("group41_var3", "Analog Output - Single-precision floating point")?
        .push("group41_var4", "Analog Output - Double-precision floating point")?
        .push("group42_var0", "Analog Output Event - Default variation")?
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
        .push("group50_var3", "Time and Date - Absolute time at last recorded time")?
        .push("group50_var4", "Time and Date - Indexed absolute time and long interval")?
        .push("group51_var1", "Time and date CTO - Absolute time, synchronized")?
        .push("group51_var2", "Time and date CTO - Absolute time, unsynchronized")?
        .push("group52_var1", "Time delay - Coarse")?
        .push("group52_var2", "Time delay - Fine")?
        .push("group60_var1", "Class objects - Class 0 data")?
        .push("group60_var2", "Class objects - Class 1 data")?
        .push("group60_var3", "Class objects - Class 2 data")?
        .push("group60_var4", "Class objects - Class 3 data")?
        .push("group70_var2", "File control - authentication")?
        .push("group70_var3", "File control - file command")?
        .push("group70_var4", "File control - file command status")?
        .push("group70_var5", "File control - file transport")?
        .push("group70_var6", "File control - file transport status")?
        .push("group70_var7", "File control - file descriptor")?
        .push("group70_var8", "File control - file specification string")?
        .push("group80_var1", "Internal Indications - Packed format")?
        .push("group102_var0", "Unsigned Integer - Default Variation")?
        .push("group102_var1", "Unsigned Integer - 8-bit")?
        .push("group110", "Octet String")?
        .push("group111", "Octet String Event")?
        .doc("Group/Variation")?
        .build()?;
    Ok(variation)
}
