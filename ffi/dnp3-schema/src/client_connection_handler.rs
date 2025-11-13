use oo_bindgen::model::{
    doc, AsynchronousInterface, BackTraced, DurationType, LibraryBuilder, Primitive, StringType,
};

pub(crate) fn define(
    lib: &mut LibraryBuilder,
    shared: &crate::shared::SharedDefinitions,
) -> BackTraced<AsynchronousInterface> {
    // Define the ConnectionInfo class first
    let connection_info = define_connection_info(lib, shared)?;
    // Define the NextEndpointAction abstract class
    let next_endpoint_action = define_next_endpoint_action(lib, shared, &connection_info)?;

    let handler = lib
        .define_interface(
            "client_connection_handler",
            "Provides fine-grained control over how TCP and TLS clients connect to endpoints"
        )?
        .begin_callback(
            "disconnected",
            "Notification that a previously successful connection failed. The task will sleep for the specified duration before attempting another connection"
        )?
        .param("addr", StringType, "Socket address that was disconnected")?
        .param("hostname", StringType, "Optional hostname (empty string if none)")?
        .returns(DurationType::Milliseconds, "Amount of time to sleep before attempting to reconnect")?
        .end_callback()?
        .begin_callback(
            "connecting",
            "Notification that a connection attempt is being made"
        )?
        .param("addr", StringType, "Socket address being connected to")?
        .param("hostname", StringType, "Optional hostname (empty string if none)")?
        .end_callback()?
        .begin_callback(
            "connect_failed",
            "Notification that connection operation failed"
        )?
        .param("addr", StringType, "Socket address that failed to connect")?
        .param("hostname", StringType, "Optional hostname (empty string if none)")?
        .end_callback()?
        .begin_callback(
            "connected",
            "Notification that a connection attempt succeeded"
        )?
        .param("addr", StringType, "Socket address that connected successfully")?
        .param("hostname", StringType, "Optional hostname (empty string if none)")?
        .end_callback()?
        .begin_callback(
            "resolution_failed",
            "Notification that DNS resolution failed"
        )?
        .param("hostname", StringType, "Hostname that failed to resolve")?
        .end_callback()?
        .begin_callback(
            "next",
            "Called to get the next endpoint to connect to or to sleep for a duration. Use the provided action object to specify the response."
        )?
        .param("action", next_endpoint_action.declaration(), "Action object to specify next endpoint or sleep duration")?
        .end_callback()?
        .build_async()?;

    Ok(handler)
}

fn define_connection_info(
    lib: &mut LibraryBuilder,
    shared: &crate::shared::SharedDefinitions,
) -> BackTraced<oo_bindgen::model::ClassHandle> {
    let connection_info = lib.declare_class("connection_info")?;

    let constructor = lib
        .define_constructor(connection_info.clone())?
        .doc("Create a new ConnectionInfo. You must call set_endpoint() before using it.")?
        .build()?;

    let set_endpoint = lib
        .define_method("set_endpoint", connection_info.clone())?
        .param(
            "endpoint",
            StringType,
            "Endpoint to connect to (hostname or IP:port)",
        )?
        .returns(
            shared.error_type.clone_enum(),
            "Error status - Ok if successful",
        )?
        .doc(doc("Set the endpoint to connect to").details(
            "This method must be called before passing the ConnectionInfo to connect_to()",
        ))?
        .build()?;

    let set_timeout = lib
        .define_method("set_timeout", connection_info.clone())?
        .param(
            "timeout_ms",
            DurationType::Milliseconds,
            "Connection timeout",
        )?
        .doc(doc("Set the connection timeout").details(
            "Optional: If not called, the operating system's default timeout will be used",
        ))?
        .build()?;

    let clear_timeout = lib
        .define_method("clear_timeout", connection_info.clone())?
        .doc("Clear any previously configured timeout so the OS default is used")?
        .build()?;

    let set_local_endpoint = lib
        .define_method("set_local_endpoint", connection_info.clone())?
        .param(
            "local_endpoint",
            StringType,
            "Local address to bind to",
        )?
        .returns(
            shared.error_type.clone_enum(),
            "Error status - Ok if successful",
        )?
        .doc(
            doc("Set the local endpoint to bind to")
                .details("Optional: If not called, the OS will select the network adapter based on the routing table for the destination address, and assign an ephemeral port.")
                .details("This is primarily useful for enforcing network segmentation in multi-homed systems, such as OT gateways that bridge device and enterprise networks. By explicitly binding to a specific adapter, you ensure traffic goes out the correct interface regardless of routing table configuration.")
                .details("Typically you should specify port 0 to let the OS assign an ephemeral port while forcing a specific network adapter. Using a specific non-zero port is rarely needed for client connections and may cause bind failures if the port is already in use.")
        )?
        .build()?;

    let clear_local_endpoint = lib
        .define_method("clear_local_endpoint", connection_info.clone())?
        .doc("Clear any previously configured local endpoint so the OS chooses the interface and port")?
        .build()?;

    let set_master_address = lib
        .define_method("set_master_address", connection_info.clone())?
        .param("address", Primitive::U16, "Master link-layer address")?
        .returns(
            shared.error_type.clone_enum(),
            "Error status - Ok if successful",
        )?
        .doc(
            doc("Change the master address for this and all subsequent connections")
                .details("Optional: The master address change is persistent and remains active across reconnections until explicitly changed again.")
                .details("This is useful when an outstation needs to communicate with different masters at different endpoints, such as during failover scenarios or when connecting to a backup master.")
                .details("If not called, the outstation uses the master address specified in its configuration.")
        )?
        .build()?;

    let destructor =
        lib.define_destructor(connection_info.clone(), "Destroy a ConnectionInfo instance")?;

    let connection_info = lib
        .define_class(&connection_info)?
        .constructor(constructor)?
        .destructor(destructor)?
        .method(set_endpoint)?
        .method(set_timeout)?
        .method(clear_timeout)?
        .method(set_local_endpoint)?
        .method(clear_local_endpoint)?
        .method(set_master_address)?
        .doc(
            doc("Builder for configuring connection parameters")
                .details("Instances are passed to {class:next_endpoint_action.connect_to()} inside {interface:client_connection_handler.next()}.")
                .details("You may construct a new instance for each callback to {interface:client_connection_handler.next()} or reuse a single instance to reduce allocations.")
                .details("When reusing, call {class:connection_info.clear_timeout()} or {class:connection_info.clear_local_endpoint()} before setting new values if you need to revert to the default timeout or adapter selection."),
        )?
        .build()?;

    Ok(connection_info)
}

fn define_next_endpoint_action(
    lib: &mut LibraryBuilder,
    shared: &crate::shared::SharedDefinitions,
    connection_info: &oo_bindgen::model::ClassHandle,
) -> BackTraced<oo_bindgen::model::ClassHandle> {
    let next_endpoint_action = lib.declare_class("next_endpoint_action")?;

    let connect_to = lib
        .define_method("connect_to", next_endpoint_action.clone())?
        .param(
            "info",
            connection_info.declaration(),
            "Connection information including endpoint and optional parameters",
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
