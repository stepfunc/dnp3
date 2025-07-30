use oo_bindgen::model::{AsynchronousInterface, BackTraced, DurationType, LibraryBuilder};

pub(crate) fn define(lib: &mut LibraryBuilder) -> BackTraced<AsynchronousInterface> {
    
    let handler = lib
        .define_interface(
            "client_connection_handler",
            "Provides fine-grained control over how TCP and TLS clients connect to endpoints"
        )?
        .begin_callback(
            "disconnected", "Notification that a previously successful connection failed. The task will sleep for the specified duration before attempting another connection"
        )?
        .returns(DurationType::Milliseconds, "Amount of time to sleep before attempting to reconnect")?
        .end_callback()?
        .build_async()?;

    Ok(handler)
}