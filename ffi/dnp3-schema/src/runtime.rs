use oo_bindgen::class::ClassDeclarationHandle;
use oo_bindgen::error_type::ErrorType;
use oo_bindgen::structs::{ConstructorType, FieldName, FunctionArgStructHandle, Number};
use oo_bindgen::types::BasicType;
use oo_bindgen::*;

fn define_runtime_config(lib: &mut LibraryBuilder) -> BindResult<FunctionArgStructHandle> {
    let num_core_threads = FieldName::new("num_core_threads");

    let config_struct = lib.declare_function_arg_struct("RuntimeConfig")?;
    lib
        .define_function_argument_struct(config_struct)?
        .add(
            &num_core_threads,
            BasicType::U16,
            doc("Number of runtime threads to spawn. For a guess of the number of CPU cores, use 0.")
                .details("Even if tons of connections are expected, it is preferred to use a value around the number of CPU cores for better performances. The library uses an efficient thread pool polling mechanism."),
        )?
        .doc("Runtime configuration")?
        .end_fields()?
        .begin_constructor("init", ConstructorType::Normal, "Initialize the configuration to default values")?
        .default(&num_core_threads, Number::U16(0))?
        .end_constructor()?
        .build()
}

pub fn define(
    lib: &mut LibraryBuilder,
    error_type: ErrorType,
) -> std::result::Result<ClassDeclarationHandle, BindingError> {
    // Forward declare the class
    let runtime_class = lib.declare_class("Runtime")?;

    let config_struct = define_runtime_config(lib)?;

    // Declare the native functions
    let new_fn = lib
        .define_function("runtime_new")
        .param(
            "config",
          config_struct,
            "Runtime configuration",
        )?
        .returns(
          runtime_class.clone(),
            "Handle to the created runtime, {null} if an error occurred",
        )?
        .fails_with(error_type)?
        .doc(
            doc("Creates a new runtime for running the protocol stack.")
            .warning("The runtime should be kept alive for as long as it's needed and it should be released with {class:Runtime.[destructor]}")
        )?
        .build()?;

    let destroy_fn = lib
        .define_function("runtime_destroy")
        .param("runtime",runtime_class.clone(), "Runtime to destroy")?
        .returns_nothing()?
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
        .custom_destroy("Shutdown")?
        .doc("Handle to the underlying runtime")?
        .build()?;

    Ok(runtime_class.declaration.clone())
}
