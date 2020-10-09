use oo_bindgen::class::ClassHandle;
use oo_bindgen::native_enum::NativeEnumHandle;
use oo_bindgen::native_function::*;
use oo_bindgen::*;

pub fn define(lib: &mut LibraryBuilder) -> Result<(ClassHandle, NativeEnumHandle), BindingError> {
    let variation_enum = lib
        .define_native_enum("Variation")?
        .push("Group1Var0", "Binary Input - Default variation")?
        .push("Group1Var1", "Binary Input - Packed format")?
        .push("Group1Var2", "Binary Input - With flags")?
        .push("Group2Var0", "Binary Input Event - Default variation")?
        .push("Group2Var1", "Binary Input Event - Without time")?
        .push("Group2Var2", "Binary Input Event - With absolute time")?
        .push("Group2Var3", "Binary Input Event - With relative time")?
        .push("Group3Var0", "Double-bit Binary Input - Default variation")?
        .push("Group3Var1", "Double-bit Binary Input - Packed format")?
        .push("Group3Var2", "Double-bit Binary Input - With flags")?
        .push(
            "Group4Var0",
            "Double-bit Binary Input Event - Default variation",
        )?
        .push("Group4Var1", "Double-bit Binary Input Event - Without time")?
        .push(
            "Group4Var2",
            "Double-bit Binary Input Event - With absolute time",
        )?
        .push(
            "Group4Var3",
            "Double-bit Binary Input Event - With relative time",
        )?
        .push("Group10Var0", "Binary Output - Default variation")?
        .push("Group10Var1", "Binary Output - Packed format")?
        .push("Group10Var2", "Binary Output - With flags")?
        .push("Group11Var0", "Binary Output Event - Default variation")?
        .push("Group11Var1", "Binary Output Event - Without time")?
        .push("Group11Var2", "Binary Output Event - With time")?
        .push(
            "Group12Var0",
            "Binary Output Command - Control Relay Output Block",
        )?
        .push(
            "Group12Var1",
            "Binary Output Command - Pattern Control Block",
        )?
        .push("Group13Var1", "Binary Output Command Event - Without time")?
        .push("Group13Var2", "Binary Output Command Event - With time")?
        .push("Group20Var0", "Counter - Default variation")?
        .push("Group20Var1", "Counter - 32-bit with flags")?
        .push("Group20Var2", "Counter - 16-bit with flags")?
        .push("Group20Var5", "Counter - 32-bit without flag")?
        .push("Group20Var6", "Counter - 16-bit without flag")?
        .push("Group21Var0", "Frozen Counter - Default variation")?
        .push("Group21Var1", "Frozen Counter - 32-bit with flags")?
        .push("Group21Var2", "Frozen Counter - 16-bit with flags")?
        .push("Group21Var5", "Frozen Counter - 32-bit with flags and time")?
        .push("Group21Var6", "Frozen Counter - 16-bit with flags and time")?
        .push("Group21Var9", "Frozen Counter - 32-bit without flag")?
        .push("Group21Var10", "Frozen Counter - 16-bit without flag")?
        .push("Group22Var0", "Counter Event - Default variation")?
        .push("Group22Var1", "Counter Event - 32-bit with flags")?
        .push("Group22Var2", "Counter Event - 16-bit with flags")?
        .push("Group22Var5", "Counter Event - 32-bit with flags and time")?
        .push("Group22Var6", "Counter Event - 16-bit with flags and time")?
        .push("Group23Var0", "Frozen Counter Event - Default variation")?
        .push("Group23Var1", "Frozen Counter Event - 32-bit with flags")?
        .push("Group23Var2", "Frozen Counter Event - 16-bit with flags")?
        .push(
            "Group23Var5",
            "Frozen Counter Event - 32-bit with flags and time",
        )?
        .push(
            "Group23Var6",
            "Frozen Counter Event - 16-bit with flags and time",
        )?
        .push("Group30Var0", "Analog Input - Default variation")?
        .push("Group30Var1", "Analog Input - 32-bit with flags")?
        .push("Group30Var2", "Analog Input - 16-bit with flags")?
        .push("Group30Var3", "Analog Input - 32-bit without flag")?
        .push("Group30Var4", "Analog Input - 16-bit without flag")?
        .push(
            "Group30Var5",
            "Analog Input - Single-precision floating point with flags",
        )?
        .push(
            "Group30Var6",
            "Analog Input - Double-precision floating point with flags",
        )?
        .push("Group32Var0", "Analog Input Event - Default variation")?
        .push("Group32Var1", "Analog Input Event - 32-bit without time")?
        .push("Group32Var2", "Analog Input Event - 16-bit without time")?
        .push("Group32Var3", "Analog Input Event - 32-bit with time")?
        .push("Group32Var4", "Analog Input Event - 16-bit with time")?
        .push(
            "Group32Var5",
            "Analog Input Event - Single-precision floating point without time",
        )?
        .push(
            "Group32Var6",
            "Analog Input Event - Double-precision floating point without time",
        )?
        .push(
            "Group32Var7",
            "Analog Input Event - Single-precision floating point with time",
        )?
        .push(
            "Group32Var8",
            "Analog Input Event - Double-precision floating point with time",
        )?
        .push("Group40Var0", "Analog Output Status - Default variation")?
        .push("Group40Var1", "Analog Output Status - 32-bit with flags")?
        .push("Group40Var2", "Analog Output Status - 16-bit with flags")?
        .push(
            "Group40Var3",
            "Analog Output Status - Single-precision floating point with flags",
        )?
        .push(
            "Group40Var4",
            "Analog Output Status - Double-precision floating point with flags",
        )?
        .push("Group41Var0", "Analog Output - Default variation")?
        .push("Group41Var1", "Analog Output - 32-bit")?
        .push("Group41Var2", "Analog Output - 16-bit")?
        .push(
            "Group41Var3",
            "Analog Output - Single-precision floating point",
        )?
        .push(
            "Group41Var4",
            "Analog Output - Double-precision floating point",
        )?
        .push("Group42Var0", "Analog Output Event - Default variation")?
        .push("Group42Var1", "Analog Output Event - 32-bit without time")?
        .push("Group42Var2", "Analog Output Event - 16-bit without time")?
        .push("Group42Var3", "Analog Output Event - 32-bit with time")?
        .push("Group42Var4", "Analog Output Event - 16-bit with time")?
        .push(
            "Group42Var5",
            "Analog Output Event - Single-precision floating point without time",
        )?
        .push(
            "Group42Var6",
            "Analog Output Event - Double-precision floating point without time",
        )?
        .push(
            "Group42Var7",
            "Analog Output Event - Single-preicions floating point with time",
        )?
        .push(
            "Group42Var8",
            "Analog Output Event - Double-preicions floating point with time",
        )?
        .push(
            "Group43Var1",
            "Analog Output Command Event - 32-bit without time",
        )?
        .push(
            "Group43Var2",
            "Analog Output Command Event - 16-bit without time",
        )?
        .push(
            "Group43Var3",
            "Analog Output Command Event - 32-bit with time",
        )?
        .push(
            "Group43Var4",
            "Analog Output Command Event - 16-bit with time",
        )?
        .push(
            "Group43Var5",
            "Analog Output Command Event - Single-precision floating point without time",
        )?
        .push(
            "Group43Var6",
            "Analog Output Command Event - Double-precision floating point without time",
        )?
        .push(
            "Group43Var7",
            "Analog Output Command Event - Single-precision floating point with time",
        )?
        .push(
            "Group43Var8",
            "Analog Output Command Event - Double-precision floating point with time",
        )?
        .push("Group50Var1", "Time and Date - Absolute time")?
        .push(
            "Group50Var3",
            "Time and Date - Absolute time at last recorded time",
        )?
        .push(
            "Group50Var4",
            "Time and Date - Indexed absolute time and long interval",
        )?
        .push(
            "Group51Var1",
            "Time and date CTO - Absolute time, synchronized",
        )?
        .push(
            "Group51Var2",
            "Time and date CTO - Absolute time, unsynchronized",
        )?
        .push("Group52Var1", "Time delay - Coarse")?
        .push("Group52Var2", "Time delay - Fine")?
        .push("Group60Var1", "Class objects - Class 0 data")?
        .push("Group60Var2", "Class objects - Class 1 data")?
        .push("Group60Var3", "Class objects - Class 2 data")?
        .push("Group60Var4", "Class objects - Class 3 data")?
        .push("Group80Var1", "Internal Indications - Packed format")?
        .push("Group110", "Octet String")?
        .push("Group111", "Octet String Event")?
        .push("Group112", "Virtual Terminal Output Block")?
        .push("Group113", "Virtual Terminal Event Data")?
        .doc("Group/Variation")?
        .build()?;

    let request = lib.declare_class("Request")?;

    let request_new_fn = lib
        .declare_native_function("request_new")?
        .return_type(ReturnType::new(
            Type::ClassRef(request.clone()),
            "Handle to the created request",
        ))?
        .doc("Create a new request")?
        .build()?;

    let request_new_class_fn = lib
        .declare_native_function("request_new_class")?
        .param("class0", Type::Bool, "Ask for class 0 (static data)")?
        .param("class1", Type::Bool, "Ask for class 1 events")?
        .param("class2", Type::Bool, "Ask for class 2 events")?
        .param("class3", Type::Bool, "Ask for class 3 events")?
        .return_type(ReturnType::new(
            Type::ClassRef(request.clone()),
            "Handle to the created request",
        ))?
        .doc(
            doc("Create a new request asking for classes")
            .details("An identical request can be created manually with {class:Request.AddAllObjectsHeader()} and variations {enum:Variation.Group60Var1}, {enum:Variation.Group60Var2}, {enum:Variation.Group60Var3} and {enum:Variation.Group60Var4}.")
        )?
        .build()?;

    let request_destroy_fn = lib
        .declare_native_function("request_destroy")?
        .param(
            "request",
            Type::ClassRef(request.clone()),
            "Request to destroy",
        )?
        .return_type(ReturnType::void())?
        .doc("Destroy a request created with {class:Request.[constructor]} or {class:Request.ClassRequest()}.")?
        .build()?;

    let request_add_one_byte_header_fn = lib
        .declare_native_function("request_add_one_byte_header")?
        .param(
            "request",
            Type::ClassRef(request.clone()),
            "Request to modify",
        )?
        .param(
            "variation",
            Type::Enum(variation_enum.clone()),
            "Variation to ask for",
        )?
        .param("start", Type::Uint8, "Start index to ask")?
        .param("stop", Type::Uint8, "Stop index to ask (inclusive)")?
        .return_type(ReturnType::void())?
        .doc("Add a one-byte start/stop variation interrogation")?
        .build()?;

    let request_add_two_byte_header_fn = lib
        .declare_native_function("request_add_two_byte_header")?
        .param(
            "request",
            Type::ClassRef(request.clone()),
            "Request to modify",
        )?
        .param(
            "variation",
            Type::Enum(variation_enum.clone()),
            "Variation to ask for",
        )?
        .param("start", Type::Uint16, "Start index to ask")?
        .param("stop", Type::Uint16, "Stop index to ask (inclusive)")?
        .return_type(ReturnType::void())?
        .doc("Add a two-byte start/stop variation interrogation")?
        .build()?;

    let request_add_all_objects_header_fn = lib
        .declare_native_function("request_add_all_objects_header")?
        .param(
            "request",
            Type::ClassRef(request.clone()),
            "Request to modify",
        )?
        .param(
            "variation",
            Type::Enum(variation_enum.clone()),
            "Variation to ask for",
        )?
        .return_type(ReturnType::void())?
        .doc("Add an all objects variation interrogation")?
        .build()?;

    let request = lib
        .define_class(&request)?
        .constructor(&request_new_fn)?
        .destructor(&request_destroy_fn)?
        .static_method("ClassRequest", &request_new_class_fn)?
        .method("AddOneByteHeader", &request_add_one_byte_header_fn)?
        .method("AddTwoByteHeader", &request_add_two_byte_header_fn)?
        .method("AddAllObjectsHeader", &request_add_all_objects_header_fn)?
        .doc(
            doc("Custom request")
            .details("Whenever a method takes a request as a parameter, the request is internally copied. Therefore, it is possible to reuse the same requests over and over.")
        )?
        .build()?;

    Ok((request, variation_enum))
}
