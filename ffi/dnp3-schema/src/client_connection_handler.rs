use oo_bindgen::model::{
    AsynchronousInterface, BackTraced, DurationType, LibraryBuilder, StringType,
};

pub(crate) fn define(
    lib: &mut LibraryBuilder,
    shared: &crate::shared::SharedDefinitions,
) -> BackTraced<AsynchronousInterface> {
    // Define the NextEndpointAction abstract class first
    let next_endpoint_action = define_next_endpoint_action(lib, shared)?;

    let handler = lib
        .define_interface(
            "client_connection_handler",
            "Provides fine-grained control over how TCP and TLS clients connect to endpoints"
        )?
        .begin_callback(
            "next", 
            "Called to get the next endpoint to connect to or to sleep for a duration. Use the provided action object to specify the response."
        )?
        .param("action", next_endpoint_action.declaration(), "Action object to specify next endpoint or sleep duration")?
        .end_callback()?
        .build_async()?;

    Ok(handler)
}

fn define_next_endpoint_action(
    lib: &mut LibraryBuilder,
    shared: &crate::shared::SharedDefinitions,
) -> BackTraced<oo_bindgen::model::ClassHandle> {
    let next_endpoint_action = lib.declare_class("next_endpoint_action")?;

    let connect_to = lib
        .define_method("connect_to", next_endpoint_action.clone())?
        .param(
            "endpoint",
            StringType,
            "Endpoint to connect to (hostname or IP:port)",
        )?
        .param(
            "timeout_ms",
            DurationType::Milliseconds,
            "Connection timeout (0 means use OS default)",
        )?
        .param(
            "local_endpoint",
            StringType,
            "Local address to bind to (empty string means any)",
        )?
        .returns(
            shared.error_type.clone_enum(),
            "Error status - Ok if successful",
        )?
        .doc("Specify the next endpoint to attempt connection to")?
        .build()?;

    let sleep_for = lib
        .define_method("sleep_for", next_endpoint_action.clone())?
        .param("duration", DurationType::Milliseconds, "Duration to sleep")?
        .returns(
            shared.error_type.clone_enum(),
            "Error status - Ok if successful",
        )?
        .doc("Indicate that the connection task should sleep for the specified duration")?
        .build()?;

    let next_endpoint_action = lib
        .define_class(&next_endpoint_action)?
        .method(connect_to)?
        .method(sleep_for)?
        .doc("Action object used by ClientConnectionHandler.next() to specify the next connection attempt or sleep duration")?
        .build()?;

    Ok(next_endpoint_action)
}
