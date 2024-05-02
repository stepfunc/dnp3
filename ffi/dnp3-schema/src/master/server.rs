use crate::shared::SharedDefinitions;
use oo_bindgen::model::*;

pub(crate) fn define_connection_handler(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
    master_channel_config: UniversalStructHandle,
    master_channel: ClassDeclarationHandle,
) -> BackTraced<AsynchronousInterface> {
    let accept_handler = define_accept_handler(lib, shared, master_channel_config.clone())?;

    let identify_link_handler =
        define_identified_link_handler(lib, shared, master_channel_config.clone())?;

    let handler = lib.define_interface(
        "connection_handler",
        "Callbacks to user code that determine how the server processes connections",
    )?
    // --- accept ---
    .begin_callback("accept", "Filter the connection solely based on the remote address")?
    .param("remote_addr", StringType, "Socket address of the remote outstation, e.g. 192.168.0.22:51532")?
    .param("acceptor", accept_handler.declaration(), "Class used to handle the accept")?
    .end_callback()?
    // --- start ---
    .begin_callback(
        "start",
        doc("Start a communication session that was previously accepted using only the socket address").warning("You must add associations and/or enable the channel from a different thread than this callback as those methods cannot be called on the Tokio runtime")
    )?
    .param("remote_addr", StringType, "Socket address of the remote outstation, e.g. 192.168.0.22:51532")?
    .param("channel", master_channel.clone(), "Class used to control the channel")?
    .end_callback()?
     // --- accept_with_link_id ---
    .begin_callback("accept_with_link_id", "Filter the connection based on the source and destination of the first link-layer frame")?
    .param("remote_addr", StringType, "Socket address of the remote outstation, e.g. 192.168.0.22:51532")?
    .param("source", Primitive::U16, "Source address from the frame")?
    .param("destination", Primitive::U16, "Destination address from the frame")?
    .param("acceptor", identify_link_handler.declaration(), "Class used to handle the accept")?
    .end_callback()?
    // --- start_with_link_id ---
    .begin_callback(
        "start_with_link_id",
        doc("Start a communication session that was previously accepted using link identity information.").warning("You must add associations and/or enable the channel from a different thread than this callback as those methods cannot be called on the Tokio runtime")
    )?
    .param("remote_addr", StringType, "Socket address of the remote outstation, e.g. 192.168.0.22:51532")?
    .param("source", Primitive::U16, "Source address from the frame")?
    .param("destination", Primitive::U16, "Destination address from the frame")?
    .param("channel", master_channel.clone(), "Class used to control the channel")?
    .end_callback()?
    .build_async()?;

    Ok(handler)
}

fn define_accept_method(
    lib: &mut LibraryBuilder,
    class: ClassDeclarationHandle,
    shared: &SharedDefinitions,
    master_channel_config: UniversalStructHandle,
) -> BackTraced<Method<Unvalidated>> {
    let accept = lib
        .define_method("accept", class.clone())?
        .doc("Accept the connection and create a master channel")?
        .param("error_mode", shared.link_error_mode.clone(), "Error mode to use for the link-layer. This should typically be {enum:link_error_mode.close}")?
        .param("config", master_channel_config, "Configuration of the channel")?
        .returns(shared.error_type.clone_enum(), "Enumeration describing the result of the operation")?
        .build()?;

    Ok(accept)
}

pub(crate) fn define_accept_handler(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
    master_channel_config: UniversalStructHandle,
) -> BackTraced<ClassHandle> {
    let handler = lib.declare_class("accept_handler")?;

    let accept = define_accept_method(lib, handler.clone(), shared, master_channel_config.clone())?;

    let get_link_identity = lib
        .define_method("get_link_identity", handler.clone())?
        .doc(
            doc("Request that server attempt to identify the outstation by reading a link-layer header from the physical layer within a timeout.")
                .details("This header is typically the beginning of an unsolicited fragment from the outstation.")
        )?
        .returns(shared.error_type.clone_enum(), "Enumeration describing the result of the operation")?
        .build()?;

    let handler = lib
        .define_class(&handler)?
        .doc("Class used to accept a connection, reject it, or defer it to link identification")?
        .method(accept)?
        .method(get_link_identity)?
        .build()?;

    Ok(handler)
}

pub(crate) fn define_identified_link_handler(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
    master_channel_config: UniversalStructHandle,
) -> BackTraced<ClassHandle> {
    let handler = lib.declare_class("identified_link_handler")?;

    let accept = define_accept_method(lib, handler.clone(), shared, master_channel_config.clone())?;

    let accept_handler = lib
        .define_class(&handler)?
        .doc("Class used to accept a connection, reject it, or defer it to link identification")?
        .method(accept)?
        .build()?;

    Ok(accept_handler)
}
