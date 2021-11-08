use oo_bindgen::class::ClassDeclarationHandle;
use oo_bindgen::doc::Unvalidated;
use oo_bindgen::error_type::ErrorType;
use oo_bindgen::name::Name;
use oo_bindgen::structs::{ConstructorType, FunctionArgStructHandle, Number};
use oo_bindgen::types::BasicType;
use oo_bindgen::*;

fn define_runtime_config(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let num_core_threads = Name::create("num_core_threads")?;

    let config_struct = lib.declare_function_arg_struct("runtime_config")?;
    let config_struct= lib
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
        .build()?;

    Ok(config_struct)
}

pub fn define(
    lib: &mut LibraryBuilder,
    error_type: ErrorType<Unvalidated>,
) -> BackTraced<ClassDeclarationHandle> {
    // Forward declare the class
    let runtime_class = lib.declare_class("runtime")?;

    let config_struct = define_runtime_config(lib)?;

    // Declare the native functions
    let new_fn = lib
        .define_function("runtime_new")?
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
            .warning("The runtime should be kept alive for as long as it's needed and it should be released with {class:runtime.[destructor]}")
        )?
        .build()?;

    let destructor = lib
        .define_destructor(
            runtime_class.clone(),
            doc("Destroy a runtime.")
                .details("This method will gracefully wait for all asynchronous operation to end before returning")
        )?;

    // Declare the object-oriented class
    let runtime_class = lib
        .define_class(&runtime_class)?
        .constructor(&new_fn)?
        .destructor(destructor)?
        .custom_destroy("shutdown")?
        .doc("Handle to the underlying runtime")?
        .build()?;

    Ok(runtime_class.declaration.clone())
}
