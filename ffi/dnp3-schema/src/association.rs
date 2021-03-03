use oo_bindgen::class::*;
use oo_bindgen::native_function::*;
use oo_bindgen::*;

use crate::shared::SharedDefinitions;

pub fn define(
    lib: &mut LibraryBuilder,
    shared_def: &SharedDefinitions,
) -> Result<ClassDeclarationHandle, BindingError> {

    let link_status_enum = lib
        .define_native_enum("LinkStatusResult")?
        .push(
            "Success",
            "The outstation responded with a valid LINK_STATUS",
        )?
        .push(
            "UnexpectedResponse",
            "There was activity on the link, but it wasn't a LINK_STATUS",
        )?
        .push(
            "TaskError",
            "The task failed for some reason (e.g. the master was shutdown)",
        )?
        .doc("Result of a link status check. See {class:Association.CheckLinkStatus()}")?
        .build()?;

    let link_status_cb = lib
        .define_one_time_callback("LinkStatusCallback", "Handler for link status check")?
        .callback("on_complete", "Called when a link status is received")?
        .param(
            "result",
            Type::Enum(link_status_enum),
            "Result of the link status",
        )?
        .return_type(ReturnType::void())?
        .build()?
        .build()?;

    let check_link_status_fn = lib
        .declare_native_function("association_check_link_status")?
        .param(
            "association",
            Type::ClassRef(association_class.clone()),
            "Association to perform the link status check",
        )?
        .param(
            "callback",
            Type::OneTimeCallback(link_status_cb),
            "Callback that will receive the result of the link status",
        )?
        .return_type(ReturnType::void())?
        .doc("Asynchronously perform a link status check")?
        .build()?;

    let association_class = lib
        .define_class(&association_class)?
        .destructor(&destroy_fn)?
        .method("AddPoll", &add_poll_fn)?
        .async_method("Read", &read_fn)?
        .async_method("Operate", &operate_fn)?
        .async_method("PerformTimeSync", &perform_time_sync_fn)?
        .async_method("ColdRestart", &cold_restart_fn)?
        .async_method("WarmRestart", &warm_restart_fn)?
        .async_method("CheckLinkStatus", &check_link_status_fn)?
        .doc("Master-outstation association to interact with")?
        .build()?;

    Ok(association_class.declaration.clone())
}
