use oo_bindgen::class::ClassDeclarationHandle;
use oo_bindgen::native_function::*;
use oo_bindgen::native_struct::*;
use oo_bindgen::*;

pub fn define(
    lib: &mut LibraryBuilder,
) -> std::result::Result<ClassDeclarationHandle, BindingError> {
    // Forward declare the class
    let runtime_class = lib.declare_class("Runtime")?;

    // Declare the C-style structs
    let config_struct = lib.declare_native_struct("RuntimeConfig")?;
    let config_struct = lib
        .define_native_struct(&config_struct)?
        .add(
            "num_core_threads",
            StructElementType::Uint16(Some(0)),
            doc("Number of runtime threads to spawn. For a guess of the number of CPU cores, use 0.")
            .details("Even if tons of connections are expected, it is preferred to use a value around the number of CPU cores for better performances. The library uses an efficient thread pool polling mechanism."),
        )?
        .doc("Runtime configuration")?
        .build()?;

    // Declare the native functions
    let new_fn = lib
        .declare_native_function("runtime_new")?
        .param(
            "config",
            Type::Struct(config_struct),
            "Runtime configuration",
        )?
        .return_type(ReturnType::new(
            Type::ClassRef(runtime_class.clone()),
            "Handle to the created runtime, {null} if an error occurred",
        ))?
        .doc(
            doc("Creates a new runtime for running the protocol stack.")
            .warning("The runtime should be kept alive for as long as it's needed and it should be released with {class:Runtime.[destructor]}")
        )?
        .build()?;

    let destroy_fn = lib
        .declare_native_function("runtime_destroy")?
        .param("runtime", Type::ClassRef(runtime_class.clone()), "Runtime to destroy")?
        .return_type(ReturnType::void())?
        .doc(
            doc("Destroy a runtime.")
            .details("This method will gracefully wait for all asynchronous operation to end before returning")
        )?
        .build()?;

    // Declare the object-oriented class
    let runtime_class = lib
        .define_class(&runtime_class)?
        .constructor(&new_fn)?
        .destructor(&destroy_fn)?
        .doc("Handle to the underlying runtime")?
        .build()?;

    Ok(runtime_class.declaration.clone())
}
