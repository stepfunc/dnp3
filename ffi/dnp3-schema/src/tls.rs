use oo_bindgen::model::{doc, BackTraced, LibraryBuilder};

pub(crate) fn define(lib: &mut LibraryBuilder) -> BackTraced<()> {
    let enable_client_name_wildcards = lib
        .define_function("enable_client_name_wildcards")?
        .doc(
            doc("Setting this option will allow the user to specify an '*' for the client's x.509 name when creating a {struct:tls_server_config}")
        )?
        .build_static("enable_client_name_wildcards")?;

    lib.define_static_class("tls_settings")?
        .static_method(enable_client_name_wildcards)?
        .doc("Global TLS configuration settings")?
        .build()?;

    Ok(())
}
