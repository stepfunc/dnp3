pub(crate) mod read_handler;
pub(crate) mod request;
pub(crate) mod server;
pub(crate) mod write_dead_band_request;

use crate::shared::SharedDefinitions;
use oo_bindgen::model::*;
use std::time::Duration;

pub(crate) fn define(lib: &mut LibraryBuilder, shared: &SharedDefinitions) -> BackTraced<()> {
    let read_handler = read_handler::define(lib, shared)?;

    let master_channel_config = define_master_channel_config(lib, shared)?;

    let master_channel_class = lib.declare_class("master_channel")?;

    let write_dead_band_request = crate::master::write_dead_band_request::define(lib)?;
    let empty_response_callback = define_empty_response_callback(lib, shared.nothing.clone())?;

    let master_channel_create_tcp_fn = lib
        .define_function("master_channel_create_tcp")?
        .param(
            "runtime",
            shared.runtime_class.clone(),
            "Runtime to use to drive asynchronous operations of the master",
        )?
        .param(
            "link_error_mode",
            shared.link_error_mode.clone(),
            "Controls how link errors are handled with respect to the TCP session",
        )?
        .param(
            "config",
            master_channel_config.clone(),
            "Generic configuration for the channel",
        )?
        .param(
            "endpoints",
            shared.endpoint_list.declaration(),
            "List of IP endpoints.",
        )?
        .param(
            "connect_strategy",
            shared.connect_strategy.clone(),
            "Controls the timing of (re)connection attempts",
        )?
        .param(
            "listener",
            shared.client_state_listener.clone(),
            "TCP connection listener used to receive updates on the status of the connection",
        )?
        .returns(
            master_channel_class.clone(),
            "Handle to the master created, {null} if an error occurred",
        )?
        .fails_with(shared.error_type.clone())?
        .doc("Create a master channel that connects to the specified TCP endpoint(s)")?
        .build_static("create_tcp_channel")?;

    let master_channel_create_tcp_2_fn = lib
        .define_function("master_channel_create_tcp_2")?
        .param(
            "runtime",
            shared.runtime_class.clone(),
            "Runtime to use to drive asynchronous operations of the master",
        )?
        .param(
            "link_error_mode",
            shared.link_error_mode.clone(),
            "Controls how link errors are handled with respect to the TCP session",
        )?
        .param(
            "config",
            master_channel_config.clone(),
            "Generic configuration for the channel",
        )?
        .param(
            "endpoints",
            shared.endpoint_list.declaration(),
            "List of IP endpoints.",
        )?
        .param(
            "connect_strategy",
            shared.connect_strategy.clone(),
            "Controls the timing of (re)connection attempts",
        )?
        .param(
            "connect_options",
            shared.connect_options.declaration(),
            "Options that control the TCP connection process",
        )?
        .param(
            "listener",
            shared.client_state_listener.clone(),
            "TCP connection listener used to receive updates on the status of the connection",
        )?
        .returns(
            master_channel_class.clone(),
            "Handle to the master created, {null} if an error occurred",
        )?
        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Create a master channel that connects to the specified TCP endpoint(s)")
                .details("This is just like {class:master_channel.create_tcp_channel()} but adds the {class:connect_options} parameter")
        )?
        .build_static("create_tcp_channel_2")?;

    let master_channel_create_serial_fn = lib
        .define_function("master_channel_create_serial")?
        .param("runtime",shared.runtime_class.clone(), "Runtime to use to drive asynchronous operations of the master")?
        .param("config",master_channel_config.clone(), "Generic configuration for the channel")?
        .param("path", StringType, "Path to the serial device. Generally /dev/tty0 on Linux and COM1 on Windows.")?
        .param("serial_params",shared.serial_port_settings.clone(), "Serial port settings")?
        .param("open_retry_delay", DurationType::Milliseconds, "Delay between attempts to open the serial port")?
        .param("listener",shared.port_state_listener.clone(), "Listener to receive updates on the status of the serial port")?
        .returns(master_channel_class.clone(), "Handle to the master created, {null} if an error occurred")?
        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Create a master channel on the specified serial port")
                .details("The returned master must be gracefully shutdown with {class:master_channel.[destructor]} when done.")
        )?
        .build_static("create_serial_channel")?;

    let master_channel_create_udp_fn = lib
        .define_function("master_channel_create_udp")?
        .param("runtime",shared.runtime_class.clone(), "Runtime to use to drive asynchronous operations of the master")?
        .param("config",master_channel_config.clone(), "Generic configuration for the channel")?
        .param("local_endpoint", StringType, "Local endpoint on which to bind the UDP socket")?
        .param("link_read_mode", shared.link_read_mode.clone(), "Determines how the link-layer parser treats frame that span datagrams. Typically set to {enum:link_read_mode.datagram}")?
        .param("retry_delay", DurationType::Milliseconds, "Amount of time to wait after a failed attempt to bind the UDP socket")?
        .returns(master_channel_class.clone(), "Handle to the master created, {null} if an error occurred")?
        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Create a UDP master channel on the local endpoint")
                .details("The returned master must be gracefully shutdown with {class:master_channel.[destructor]} when done.")
        )?
        .build_static("create_udp_channel")?;

    let channel_destructor = lib.define_destructor(
        master_channel_class.clone(),
        "Shutdown a {class:master_channel} and release all resources",
    )?;

    let master_channel_create_tls_fn = lib
        .define_function("master_channel_create_tls")?
        .param(
            "runtime",
            shared.runtime_class.clone(),
            "Runtime to use to drive asynchronous operations of the master",
        )?
        .param(
            "link_error_mode",
            shared.link_error_mode.clone(),
            "Controls how link errors are handled with respect to the TCP session",
        )?
        .param(
            "config",
            master_channel_config.clone(),
            "Generic configuration for the channel",
        )?
        .param(
            "endpoints",
            shared.endpoint_list.declaration(),
            "List of IP endpoints.",
        )?
        .param(
            "connect_strategy",
            shared.connect_strategy.clone(),
            "Controls the timing of (re)connection attempts",
        )?
        .param(
            "listener",
            shared.client_state_listener.clone(),
            "TCP connection listener used to receive updates on the status of the connection",
        )?
        .param(
            "tls_config",
            shared.tls_client_config.clone(),
            "TLS client configuration",
        )?
        .returns(
            master_channel_class.clone(),
            "Handle to the master created, {null} if an error occurred",
        )?
        .fails_with(shared.error_type.clone())?
        .doc("Create a master channel that connects to the specified TCP endpoint(s) and establish a TLS session with the remote.")?
        .build_static("create_tls_channel")?;

    let master_channel_create_tls_2_fn = lib
        .define_function("master_channel_create_tls_2")?
        .param(
            "runtime",
            shared.runtime_class.clone(),
            "Runtime to use to drive asynchronous operations of the master",
        )?
        .param(
            "link_error_mode",
            shared.link_error_mode.clone(),
            "Controls how link errors are handled with respect to the TCP session",
        )?
        .param(
            "config",
            master_channel_config.clone(),
            "Generic configuration for the channel",
        )?
        .param(
            "endpoints",
            shared.endpoint_list.declaration(),
            "List of IP endpoints.",
        )?
        .param(
            "connect_strategy",
            shared.connect_strategy.clone(),
            "Controls the timing of (re)connection attempts",
        )?
        .param(
            "connect_options",
            shared.connect_options.declaration(),
            "Options that control the TCP connection process",
        )?
        .param(
            "listener",
            shared.client_state_listener.clone(),
            "TCP connection listener used to receive updates on the status of the connection",
        )?
        .param(
            "tls_config",
            shared.tls_client_config.clone(),
            "TLS client configuration",
        )?
        .returns(
            master_channel_class.clone(),
            "Handle to the master created, {null} if an error occurred",
        )?
        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Create a master channel that connects to the specified TCP endpoint(s) and establish a TLS session with the remote.")
                .details("This is just like {class:master_channel.create_tls_channel()} but adds the {class:connect_options} parameter")
        )?
        .build_static("create_tls_channel_2")?;

    let enable_method = lib
        .define_method("enable", master_channel_class.clone())?
        .fails_with(shared.error_type.clone())?
        .doc("start communications")?
        .build()?;

    let disable_method = lib
        .define_method("disable", master_channel_class.clone())?
        .fails_with(shared.error_type.clone())?
        .doc("stop communications")?
        .build()?;

    let association_id = define_association_id(lib)?;
    let poll_id = define_poll_id(lib)?;

    let association_config = define_association_config(lib, shared)?;

    let association_handler_interface = define_association_handler(lib)?;

    let association_information_interface = define_association_information(lib, shared)?;

    let request_class = crate::master::request::define(lib, shared)?;

    let add_association_method = lib
        .define_method("add_association", master_channel_class.clone())?
        .param(
            "address",
            Primitive::U16,
            "DNP3 data-link address of the remote outstation",
        )?
        .param(
            "config",
            association_config.clone(),
            "Association configuration",
        )?
        .param(
            "read_handler",
            read_handler.clone(),
            "Interface uses to load measurement data",
        )?
        .param(
            "association_handler",
            association_handler_interface.clone(),
            "Association specific callbacks such as time synchronization",
        )?
        .param(
            "association_information",
            association_information_interface.clone(),
            "Association information interface",
        )?
        .returns(association_id.clone(), "Id of the association")?
        .fails_with(shared.error_type.clone())?
        .doc("Add an association to the channel")?
        .build()?;

    let add_udp_association_method = lib
        .define_method("add_udp_association", master_channel_class.clone())?
        .param(
            "address",
            Primitive::U16,
            "DNP3 data-link address of the remote outstation",
        )?
        .param(
            "destination",
            StringType,
            "IP address and port of the outstation",
        )?
        .param("config", association_config, "Association configuration")?
        .param(
            "read_handler",
            read_handler.clone(),
            "Interface uses to load measurement data",
        )?
        .param(
            "association_handler",
            association_handler_interface,
            "Association specific callbacks such as time synchronization",
        )?
        .param(
            "association_information",
            association_information_interface,
            "Association information interface",
        )?
        .returns(association_id.clone(), "Id of the association")?
        .fails_with(shared.error_type.clone())?
        .doc("Add a UDP association to the channel")?
        .build()?;

    let remove_association_method = lib
        .define_method("remove_association", master_channel_class.clone())?
        .param("id", association_id.clone(), "Id of the association")?
        .fails_with(shared.error_type.clone())?
        .doc("Remove an association from the channel")?
        .build()?;

    let add_poll_method = lib.define_method("add_poll", master_channel_class.clone())?
        .param("id", association_id.clone(), "Association on which to add the poll")?
        .param("request", request_class.declaration(), "Request to perform")?
        .param("period", DurationType::Milliseconds, "Period to wait between each poll (in ms)")?
        .returns(poll_id.clone(), "Id of the created poll")?
        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Add a periodic poll to an association")
                .details("Each result of the poll will be sent to the {interface:read_handler} of the association.")
        )?
        .build()?;

    let remove_poll_method = lib
        .define_method("remove_poll", master_channel_class.clone())?
        .param("poll_id", poll_id.clone(), "Id of the created poll")?

        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Add a periodic poll to an association")
                .details("Each result of the poll will be sent to the {interface:read_handler} of the association.")
        )?
        .build()?;

    let demand_poll_method = lib.define_method("demand_poll", master_channel_class.clone())?
        .param("poll_id", poll_id, "Id of the poll")?

        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Demand the immediate execution of a poll previously created with {class:master_channel.add_poll()}.")
                .details("This method returns immediately. The result will be sent to the registered {interface:read_handler}.")
                .details("This method resets the internal timer of the poll.")
        )?
        .build()?;

    let set_decode_level_method = lib
        .define_method("set_decode_level", master_channel_class.clone())?
        .param(
            "decode_level",
            shared.decode_level.clone(),
            "Decoding level",
        )?
        .fails_with(shared.error_type.clone())?
        .doc("Set the decoding level for the channel")?
        .build()?;

    let get_decode_level_method = lib
        .define_method("get_decode_level", master_channel_class.clone())?
        .returns(shared.decode_level.clone(), "Decode level")?
        .fails_with(shared.error_type.clone())?
        .doc("Get the decoding level for the channel")?
        .build()?;

    let read_callback = define_read_callback(lib, shared.nothing.clone())?;

    let read_async = lib
        .define_future_method("read", master_channel_class.clone(), read_callback.clone())?
        .param("association",association_id.clone(), "Association on which to perform the read")?
        .param("request",request_class.declaration(), "Request to send")?
        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Perform a read on the association.")
                .details("The callback will be called once the read is completely received, but the actual values will be sent to the {interface:read_handler} of the association.")
        )?
        .build()?;

    let write_dead_bands_async = lib
        .define_future_method(
            "write_dead_bands",
            master_channel_class.clone(),
            empty_response_callback.clone(),
        )?
        .param(
            "association",
            association_id.clone(),
            "Association on which to perform the WRITE",
        )?
        .param(
            "request",
            write_dead_band_request.declaration(),
            "Request containing headers of analog input dead-bands (group 34)",
        )?
        .fails_with(shared.error_type.clone())?
        .doc(doc(
            "Perform a WRITE on the association using the supplied collection of dead-band headers",
        ))?
        .build()?;

    let send_and_expect_empty_response = lib
        .define_future_method(
            "send_and_expect_empty_response",
            master_channel_class.clone(),
            empty_response_callback,
        )?
        .param(
            "association",
            association_id.clone(),
            "Association on which to perform the request",
        )?
        .param(
            "function",
            shared.function_code.clone(),
            "Function code for the request",
        )?
        .param(
            "headers",
            request_class.declaration(),
            "Headers that will be contained in the request",
        )?
        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Send the specified request to the association using the supplied function and collection of request headers")
                .details("This is useful for constructing various types of WRITE and FREEZE operations where an empty response is expected from the outstation, and the only indication of success are bits in IIN.2.")
                .details("The request will fail if 1) The response contains object headers or 2) One of the error bits in IIN.2 is set.")
        )?
        .build()?;

    let read_with_handler_async = lib
        .define_future_method("read_with_handler", master_channel_class.clone(), read_callback)?
        .param("association",association_id.clone(), "Association on which to perform the read")?
        .param("request",request_class.declaration(), "Request to send")?
        .param("handler",read_handler, "Custom {interface:read_handler} to send the data to")?
        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Perform a read on the association.")
                .details("The callback will be called once the read is completely received, but the actual values will be sent to the {interface:read_handler} passed as a parameter.")
        )?
        .build()?;

    let command = define_command_builder(lib, shared)?;
    let command_mode = define_command_mode(lib)?;
    let command_cb = define_command_callback(lib, shared.nothing.clone())?;

    let operate_async = lib
        .define_future_method("operate", master_channel_class.clone(), command_cb)?
        .param(
            "association",
            association_id.clone(),
            "Id of the association",
        )?
        .param("mode", command_mode, "Operation mode")?
        .param("command", command.declaration(), "Command to send")?
        .fails_with(shared.error_type.clone())?
        .doc("Asynchronously perform a command operation on the association")?
        .build()?;

    let time_sync_mode = define_time_sync_mode(lib)?;
    let time_sync_cb = define_time_sync_callback(lib, shared.nothing.clone())?;

    let perform_time_sync_async = lib
        .define_future_method(
            "synchronize_time",
            master_channel_class.clone(),
            time_sync_cb,
        )?
        .param(
            "association",
            association_id.clone(),
            "Id of the association",
        )?
        .param("mode", time_sync_mode, "Time sync mode")?
        .fails_with(shared.error_type.clone())?
        .doc("Asynchronously perform a time sync operation to the association")?
        .build()?;

    let restart_cb = define_restart_callback(lib)?;

    let cold_restart_async = lib
        .define_future_method(
            "cold_restart",
            master_channel_class.clone(),
            restart_cb.clone(),
        )?
        .param(
            "association",
            association_id.clone(),
            "Id of the association",
        )?
        .fails_with(shared.error_type.clone())?
        .doc("Asynchronously perform a cold restart operation to the association")?
        .build()?;

    let warm_restart_async = lib
        .define_future_method("warm_restart", master_channel_class.clone(), restart_cb)?
        .param(
            "association",
            association_id.clone(),
            "Id of the association",
        )?
        .fails_with(shared.error_type.clone())?
        .doc("Asynchronously perform a warm restart operation to the association")?
        .build()?;

    let file_info_cb = define_file_info_callback(lib, shared)?;
    let file_reader = define_file_reader(lib, shared)?;
    let file_read_config = define_file_read_config(lib)?;
    let dir_read_config = define_dir_read_config(lib)?;

    let read_file = lib
        .define_method("read_file", master_channel_class.clone())?
        .doc("Start an operation to READ a file from the outstation using a {interface:file_reader} to receive data")?
        .param(
            "association",
            association_id.clone(),
            "Id of the association",
        )?
        .param("remote_file_path", StringType, "Path of the remote file")?
        .param("config", file_read_config.clone(), "Configuration for the read operation")?
        .param("reader", file_reader.clone(), "Interface used to receive file data")?
        .fails_with(shared.error_type.clone())?
        .build()?;

    let read_file_with_auth = lib
        .define_method("read_file_with_auth", master_channel_class.clone())?
        .doc(
            doc("Start an operation to READ a file from the outstation using a {interface:file_reader} to receive data")
                .details("This variant first requests an authentication key from the outstation using the supplied credentials")
        )?
        .param(
            "association",
            association_id.clone(),
            "Id of the association",
        )?
        .param("remote_file_path", StringType, "Path of the remote file")?
        .param("config", file_read_config, "Configuration for the read operation")?
        .param("reader", file_reader, "Interface used to receive file data")?
        .param("user_name", StringType, "User name sent to the outstation")?
        .param("password", StringType, "Password sent to the outstation")?
        .fails_with(shared.error_type.clone())?
        .build()?;

    let get_file_auth_key_async = lib
        .define_future_method(
            "get_file_auth_key",
            master_channel_class.clone(),
            shared.file.file_auth_cb.clone(),
        )?
        .param(
            "association",
            association_id.clone(),
            "Id of the association",
        )?
        .param("username", StringType, "User name")?
        .param("password", StringType, "Password")?
        .fails_with(shared.error_type.clone())?
        .doc("Obtain a file authentication key")?
        .build()?;

    let open_file_async = lib
        .define_future_method("open_file", master_channel_class.clone(), shared.file.file_open_cb.clone())?
        .param(
            "association",
            association_id.clone(),
            "Id of the association",
        )?
        .param("file_name", StringType, "Complete path to the remote file")?
        .param("auth_key", Primitive::U32, "Optional authentication key (0 == None)")?
        .param("permissions", shared.file.permissions.clone(), "Permissions sent in the file open request")?
        .param(
            "file_size",
            Primitive::U32,
            "File size sent in the request (zero when reading). When writing use the actual file size or 0xFFFFFFFF to indicate the size is unknown"
        )?
        .param("file_mode", shared.file.file_mode.clone(), "Mode used to open the file")?
        .param("max_block_size", Primitive::U16, "Requested maximum block size")?
        .fails_with(shared.error_type.clone())?
        .doc("Asynchronously open a file")?
        .build()?;

    let write_file_block_async = lib
        .define_future_method("write_file_block", master_channel_class.clone(), shared.file.file_op_cb.clone())?
        .param(
            "association",
            association_id.clone(),
            "Id of the association",
        )?
        .param("handle", Primitive::U32, "Handle returned when the file was opened")?
        .param("block_number", Primitive::U32, "Sequential block number in the range [0 .. 0x7FFFFFFF]. The top bit is indicates the final block")?
        .param("final_block", Primitive::Bool, "Indicate that this is the final block")?
        .param("block_data", shared.byte_collection.clone(), "Collection of bytes. This must not be larger than the maximum block size returned by the outstation")?
        .fails_with(shared.error_type.clone())?
        .doc("Asynchronously write a block of file data to the outstation")?
        .build()?;

    let close_file_async = lib
        .define_future_method(
            "close_file",
            master_channel_class.clone(),
            shared.file.file_op_cb.clone(),
        )?
        .param(
            "association",
            association_id.clone(),
            "Id of the association",
        )?
        .param(
            "handle",
            Primitive::U32,
            "Handle returned when the file was opened",
        )?
        .fails_with(shared.error_type.clone())?
        .doc("Asynchronously close a file")?
        .build()?;

    let get_file_info_async = lib
        .define_future_method("get_file_info", master_channel_class.clone(), file_info_cb)?
        .param(
            "association",
            association_id.clone(),
            "Id of the association",
        )?
        .param("file_name", StringType, "Complete path to the remote file")?
        .fails_with(shared.error_type.clone())?
        .doc("Asynchronously retrieve information on a particular file")?
        .build()?;

    let file_info_iter =
        lib.define_iterator("file_info_iterator", shared.file.file_info.clone())?;

    let read_directory_cb = lib.define_future_interface(
        "read_directory_callback",
        "Callback interface for retrieving a directory list",
        file_info_iter,
        "{iterator} of {struct:file_info} values",
        shared.file.file_error.clone(),
    )?;

    let read_directory_async = lib
        .define_future_method(
            "read_directory",
            master_channel_class.clone(),
            read_directory_cb.clone(),
        )?
        .param(
            "association",
            association_id.clone(),
            "Id of the association",
        )?
        .param(
            "dir_path",
            StringType,
            "Complete path to the remote directory",
        )?
        .param(
            "config",
            dir_read_config.clone(),
            "Configuration for the directory read operation",
        )?
        .fails_with(shared.error_type.clone())?
        .doc("Asynchronously retrieve a directory listing")?
        .build()?;

    let read_directory_with_auth_async = lib
        .define_future_method(
            "read_directory_with_auth",
            master_channel_class.clone(),
            read_directory_cb,
        )?
        .param(
            "association",
            association_id.clone(),
            "Id of the association",
        )?
        .param(
            "dir_path",
            StringType,
            "Complete path to the remote directory",
        )?
        .param(
            "config",
            dir_read_config,
            "Configuration for the directory read operation",
        )?
        .param("user_name", StringType, "User name sent to the outstation")?
        .param("password", StringType, "Password sent to the outstation")?
        .fails_with(shared.error_type.clone())?
        .doc(
            "Asynchronously retrieve a directory listing by first obtaining an authentication key",
        )?
        .build()?;

    let link_status_cb = define_link_status_callback(lib, shared.nothing.clone())?;

    let check_link_status_async = lib
        .define_future_method(
            "check_link_status",
            master_channel_class.clone(),
            link_status_cb,
        )?
        .param("association", association_id, "Id of the association")?
        .fails_with(shared.error_type.clone())?
        .doc("Asynchronously perform a link status check")?
        .build()?;

    let master_channel_class = lib.define_class(&master_channel_class)?
        .destructor(channel_destructor)?
        .static_method(master_channel_create_tcp_fn)?
        .static_method(master_channel_create_tcp_2_fn)?
        .static_method(master_channel_create_tls_fn)?
        .static_method(master_channel_create_tls_2_fn)?
        .static_method(master_channel_create_serial_fn)?
        .static_method(master_channel_create_udp_fn)?
        .method(enable_method)?
        .method(disable_method)?
        .method(add_association_method)?
        .method(add_udp_association_method)?
        .method(remove_association_method)?
        .method(set_decode_level_method)?
        .method(get_decode_level_method)?
        .method(add_poll_method)?
        .method(remove_poll_method)?
        .method(demand_poll_method)?
        .method(read_file)?
        .method(read_file_with_auth)?
        .async_method(read_async)?
        .async_method(read_with_handler_async)?
        .async_method(operate_async)?
        .async_method(perform_time_sync_async)?
        .async_method(cold_restart_async)?
        .async_method(warm_restart_async)?
        .async_method(write_dead_bands_async)?
        .async_method(send_and_expect_empty_response)?
        .async_method(get_file_info_async)?
        .async_method(get_file_auth_key_async)?
        .async_method(open_file_async)?
        .async_method(write_file_block_async)?
        .async_method(close_file_async)?
        .async_method(read_directory_async)?
        .async_method(read_directory_with_auth_async)?
        .async_method(check_link_status_async)?
        .custom_destroy("shutdown")?
        .doc(
            doc("Represents a communication channel for a master station")
                .details("To communicate with a particular outstation, you need to add an association with {class:master_channel.add_association()}.")
                .warning("The class methods that return a value (e.g. as {class:master_channel.add_association()}) cannot be called from within a callback.")
        )?
        .build()?;

    server::define_server_components(
        lib,
        shared,
        master_channel_config.clone(),
        master_channel_class.declaration(),
    )?;

    Ok(())
}

fn define_association_id(lib: &mut LibraryBuilder) -> BackTraced<UniversalStructHandle> {
    let id = lib.declare_universal_struct("association_id")?;
    let id = lib
        .define_opaque_struct(id)?
        .add(
            "address",
            Primitive::U16,
            "Outstation address of the association",
        )?
        .doc("Association identifier")?
        .end_fields()?
        .build()?;

    Ok(id)
}

fn define_poll_id(lib: &mut LibraryBuilder) -> BackTraced<UniversalStructHandle> {
    let id = lib.declare_universal_struct("poll_id")?;
    let id = lib
        .define_opaque_struct(id)?
        .add(
            "association_id",
            Primitive::U16,
            "Outstation address of the association",
        )?
        .add(
            "id",
            Primitive::U64,
            "Unique poll id assigned by the association",
        )?
        .doc("Poll identifier")?
        .end_fields()?
        .build()?;

    Ok(id)
}

fn define_read_callback(
    lib: &mut LibraryBuilder,
    nothing: EnumHandle,
) -> BackTraced<FutureInterface<Unvalidated>> {
    let read_error = lib
        .define_error_type(
            "read_error",
            "read_exception",
            ExceptionType::CheckedException,
        )?
        .add_task_errors()?
        .doc("Errors that can occur during a read operation")?
        .build()?;

    let callback = lib.define_future_interface(
        "read_task_callback",
        "Handler for read tasks",
        nothing,
        "Result of the read task",
        read_error,
    )?;

    Ok(callback)
}

fn define_association_config(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> BackTraced<FunctionArgStructHandle> {
    let event_classes = define_event_classes(lib)?;
    let classes = define_classes(lib)?;

    let auto_time_sync_enum = lib
        .define_enum("auto_time_sync")?
        .push("none", "Do not perform automatic time sync")?
        .push(
            "lan",
            "Perform automatic time sync with Record Current Time (0x18) function code",
        )?
        .push(
            "non_lan",
            "Perform automatic time sync with Delay Measurement (0x17) function code",
        )?
        .doc("Automatic time synchronization configuration")?
        .build()?;

    let auto_time_sync = Name::create("auto_time_sync")?;
    let auto_tasks_retry_strategy = Name::create("auto_tasks_retry_strategy")?;
    let keep_alive_timeout = Name::create("keep_alive_timeout")?;
    let auto_integrity_scan_on_buffer_overflow =
        Name::create("auto_integrity_scan_on_buffer_overflow")?;
    let max_queued_user_requests = Name::create("max_queued_user_requests")?;
    let response_timeout = Name::create("response_timeout")?;
    let association_config = lib.declare_function_argument_struct("association_config")?;

    let association_config = lib
        .define_function_argument_struct(association_config)?
        .doc("Association configuration")?
        .add(
            response_timeout.clone(),
            DurationType::Milliseconds,
            "Timeout for receiving a response on this association"
        )?
        .add(
            "disable_unsol_classes",
            event_classes.clone(),
            "Classes to disable unsolicited responses at startup",
        )?
        .add(
            "enable_unsol_classes",
            event_classes.clone(),
            "Classes to enable unsolicited responses at startup",
        )?
        .add(
            "startup_integrity_classes",
            classes,
            doc("Startup integrity classes to ask on master startup and when an outstation restart is detected.").details("For conformance, this should be Class 1230.")
        )?
        .add(
            &auto_time_sync,
            auto_time_sync_enum,
            "Automatic time synchronization configuration",
        )?
        .add(
            &auto_tasks_retry_strategy,
            shared.retry_strategy.clone(),
            "Automatic tasks retry strategy",
        )?
        .add(&keep_alive_timeout,
             DurationType::Seconds,
             doc("Delay of inactivity before sending a REQUEST_LINK_STATUS to the outstation").details("A value of zero means no automatic keep-alive.")
        )?
        .add(&auto_integrity_scan_on_buffer_overflow,
             Primitive::Bool,
             doc("Automatic integrity scan when an EVENT_BUFFER_OVERFLOW is detected")
        )?
        .add("event_scan_on_events_available",
             event_classes,
             doc("Classes to automatically send reads when the IIN bit is asserted")
        )?
        .add(&max_queued_user_requests,
             Primitive::U16,
             doc("maximum number of user requests (e.g. commands, adhoc reads, etc) that will be queued before back-pressure is applied by failing requests")
        )?
        .end_fields()?
        .begin_initializer("init", InitializerType::Normal, "Initialize the configuration with the specified values")?
        .default(&response_timeout, Duration::from_secs(5))?
        .default_variant(&auto_time_sync, "none")?
        .default_struct(&auto_tasks_retry_strategy)?
        .default(&keep_alive_timeout, Duration::from_secs(60))?
        .default(&auto_integrity_scan_on_buffer_overflow, true)?
        .default(&max_queued_user_requests, NumberValue::U16(16))?
        .end_initializer()?
        .build()?;

    Ok(association_config)
}

fn define_master_channel_config(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> BackTraced<UniversalStructHandle> {
    let config = lib.declare_universal_struct("master_channel_config")?;

    let decode_level = Name::create("decode_level")?;

    let tx_buffer_size = Name::create("tx_buffer_size")?;
    let rx_buffer_size = Name::create("rx_buffer_size")?;

    let config = lib.define_universal_struct(config)?
        .doc("Configuration for a MasterChannel that is independent of the physical layer")?
        .add("address", Primitive::U16, "Local DNP3 data-link address")?
        .add(decode_level.clone(), shared.decode_level.clone(), "Decoding level for this master. You can modify this later on with {class:master_channel.set_decode_level()}.")?
        .add(tx_buffer_size.clone(), Primitive::U16, doc("TX buffer size").details("Must be at least 249"))?
        .add(rx_buffer_size.clone(), Primitive::U16, doc("RX buffer size").details("Must be at least 2048"))?
        .end_fields()?
        .begin_initializer("init", InitializerType::Normal, "Initialize {struct:master_channel_config} to default values")?
        .default_struct(&decode_level)?
        .default(&tx_buffer_size, NumberValue::U16(2048))?
        .default(&rx_buffer_size, NumberValue::U16(2048))?
        .end_initializer()?
        .build()?;

    Ok(config)
}

fn define_utc_timestamp(lib: &mut LibraryBuilder) -> BackTraced<UniversalStructHandle> {
    let value = Name::create("value")?;
    let is_valid = Name::create("is_valid")?;

    let timestamp_utc = lib.declare_universal_struct("utc_timestamp")?;
    let timestamp_utc = lib.define_universal_struct(timestamp_utc)?
        .add(&value, Primitive::U64, doc("Count of milliseconds since UNIX epoch").warning("Only the lower 48-bits are used in DNP3 timestamps and time synchronization"))?
        .add(&is_valid, Primitive::Bool, "True if the timestamp is valid, false otherwise.")?
        .doc(doc("Timestamp value returned by {interface:association_handler.get_current_time()}.").details("{struct:utc_timestamp.value} is only valid if {struct:utc_timestamp.is_valid} is true."))?
        .end_fields()?
        .begin_initializer("valid", InitializerType::Static, "Construct a valid {struct:utc_timestamp}")?
        .default(&is_valid, true)?
        .end_initializer()?
        .begin_initializer("invalid", InitializerType::Static, "Construct an invalid {struct:utc_timestamp}")?
        .default(&is_valid, false)?
        .default(&value, NumberValue::U64(0))?
        .end_initializer()?
        .build()?;

    Ok(timestamp_utc)
}

fn define_association_handler(lib: &mut LibraryBuilder) -> BackTraced<AsynchronousInterface> {
    let timestamp_utc = define_utc_timestamp(lib)?;

    let timestamp_utc = lib.define_interface(
        "association_handler",
        "Callbacks for a particular outstation association",
    )?
        .begin_callback(
            "get_current_time",
            doc("Returns the current time or an invalid time if none is available")
                .details("This callback is used when the master performs time synchronization for a particular outstation.")
                .details("This could return the system clock or some other clock's time"),
        )?
        .returns(
            timestamp_utc,
            "The current time",
        )?
        .end_callback()?
        .build_async()?;

    Ok(timestamp_utc)
}

fn define_association_information(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> BackTraced<AsynchronousInterface> {
    let task_type = lib
        .define_enum("task_type")?
        .push("user_read", "User-defined read request")?
        .push("periodic_poll", "Periodic poll task")?
        .push("startup_integrity", "Startup integrity scan")?
        .push(
            "auto_event_scan",
            "Automatic event scan caused by RESTART IIN bit detection",
        )?
        .push("command", "Command request")?
        .push("clear_restart_bit", "Clear RESTART IIN bit")?
        .push("enable_unsolicited", "Enable unsolicited startup request")?
        .push("disable_unsolicited", "Disable unsolicited startup request")?
        .push("time_sync", "Time synchronisation task")?
        .push("restart", "Cold or warm restart task")?
        .push("write_dead_bands", "Write analog input dead-bands")?
        .push(
            "generic_empty_response",
            "Generic request that expects an empty response",
        )?
        .push("file_read", "Read a file from the outstation")?
        .push("get_file_info", "Get information about a file")?
        .push(
            "file_auth",
            "Send username and password and get back an auth key from the outstation",
        )?
        .push("file_open", "Open a file on the outstation")?
        .push("file_write_block", "Write a file block to the outstation")?
        .push("file_close", "Close a file on the outstation")?
        .doc("Task type used in {interface:association_information}")?
        .build()?;

    let task_error = lib
        .define_enum("task_error")?
        .add_task_errors()?
        .doc("Task error used in {interface:association_information}")?
        .build()?;

    let handle = lib
        .define_interface(
            "association_information",
            "Informational callbacks about the current state of an outstation association",
        )?
        .begin_callback("task_start", "Called when a new task is started")?
        .param(
            "task_type",
            task_type.clone(),
            "Type of task that was started",
        )?
        .param(
            "function_code",
            shared.function_code.clone(),
            "Function code used by the task",
        )?
        .param("seq", Primitive::U8, "Sequence number of the request")?
        .end_callback()?
        .begin_callback("task_success", "Called when a task successfully completes")?
        .param(
            "task_type",
            task_type.clone(),
            "Type of task that was completed",
        )?
        .param(
            "function_code",
            shared.function_code.clone(),
            "Function code used by the task",
        )?
        .param("seq", Primitive::U8, "Sequence number of the response that completed the request. This will typically be the same as the seq number in the request, except for READ requests where the response is multi-fragmented."
        )?
        .end_callback()?
        .begin_callback("task_fail", "Called when a task fails")?
        .param("task_type", task_type, "Type of task that was completed")?
        .param("error", task_error, "Error that prevented ")?
        .end_callback()?
        .begin_callback(
            "unsolicited_response",
            "Called when an unsolicited response is received",
        )?
        .param(
            "is_duplicate",
            Primitive::Bool,
            "Is the unsolicited response a duplicate response",
        )?
        .param("seq", Primitive::U8, "Sequence number of the response")?
        .end_callback()?
        .build_async()?;

    Ok(handle)
}

fn define_event_classes(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let class1 = Name::create("class1")?;
    let class2 = Name::create("class2")?;
    let class3 = Name::create("class3")?;

    let event_classes = lib.declare_function_argument_struct("event_classes")?;
    let event_classes = lib
        .define_function_argument_struct(event_classes)?
        .add(&class1, Primitive::Bool, "Class 1 events")?
        .add(&class2, Primitive::Bool, "Class 2 events")?
        .add(&class3, Primitive::Bool, "Class 3 events")?
        .doc("Event classes")?
        .end_fields()?
        .add_full_initializer("init")?
        .begin_initializer(
            "all",
            InitializerType::Static,
            "Initialize all classes to true",
        )?
        .default(&class1, true)?
        .default(&class2, true)?
        .default(&class3, true)?
        .end_initializer()?
        .begin_initializer(
            "none",
            InitializerType::Static,
            "Initialize all classes to false",
        )?
        .default(&class1, false)?
        .default(&class2, false)?
        .default(&class3, false)?
        .end_initializer()?
        .build()?;

    Ok(event_classes)
}

fn define_classes(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let class0 = Name::create("class0")?;
    let class1 = Name::create("class1")?;
    let class2 = Name::create("class2")?;
    let class3 = Name::create("class3")?;

    let classes = lib.declare_function_argument_struct("classes")?;
    let classes = lib
        .define_function_argument_struct(classes)?
        .add(&class0, Primitive::Bool, "Class 0 (static data)")?
        .add(&class1, Primitive::Bool, "Class 1 events")?
        .add(&class2, Primitive::Bool, "Class 2 events")?
        .add(&class3, Primitive::Bool, "Class 3 events")?
        .doc("Class 0, 1, 2 and 3 config")?
        .end_fields()?
        .add_full_initializer("init")?
        .begin_initializer(
            "all",
            InitializerType::Static,
            "Initialize all classes to true",
        )?
        .default(&class0, true)?
        .default(&class1, true)?
        .default(&class2, true)?
        .default(&class3, true)?
        .end_initializer()?
        .begin_initializer(
            "none",
            InitializerType::Static,
            "Initialize all classes to false",
        )?
        .default(&class0, false)?
        .default(&class1, false)?
        .default(&class2, false)?
        .default(&class3, false)?
        .end_initializer()?
        .build()?;

    Ok(classes)
}

fn define_command_mode(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let mode = lib
        .define_enum("command_mode")?
        .push("direct_operate", "Perform a Direct Operate (0x05)")?
        .push(
            "select_before_operate",
            "Perform a Select and Operate (0x03 then 0x04)",
        )?
        .doc("Command operation mode")?
        .build()?;

    Ok(mode)
}

const TASK_ERRORS: &[(&str, &str)] = &[
    ("too_many_requests", "too many user requests queued"),
    ("iin_error", "outstation returned an IIN.2 error bit"),
    (
        "bad_response",
        "response was malformed or contained object headers",
    ),
    (
        "response_timeout",
        "timeout occurred before receiving a response",
    ),
    (
        "write_error",
        "insufficient buffer space to serialize the request",
    ),
    ("no_connection", "no connection"),
    ("shutdown", "master was shutdown"),
    ("association_removed", "association was removed mid-task"),
    ("bad_encoding", "request data could not be encoded"),
];

pub(crate) trait TaskErrors: Sized {
    fn add_task_errors(self) -> BackTraced<Self>;
}

impl TaskErrors for ErrorTypeBuilder<'_> {
    fn add_task_errors(self) -> BackTraced<Self> {
        let mut builder = self;
        for (name, doc) in TASK_ERRORS {
            builder = builder.add_error(name, doc)?;
        }

        Ok(builder)
    }
}

impl TaskErrors for EnumBuilder<'_> {
    fn add_task_errors(self) -> BackTraced<Self> {
        let mut builder = self;
        for (name, doc) in TASK_ERRORS {
            builder = builder.push(name, doc)?;
        }

        Ok(builder)
    }
}

fn define_command_callback(
    lib: &mut LibraryBuilder,
    nothing: EnumHandle,
) -> BackTraced<FutureInterface<Unvalidated>> {
    let command_error = lib
        .define_error_type(
            "command_error",
            "command_exception",
            ExceptionType::CheckedException,
        )?
        .add_error(
            "bad_status",
            "Outstation indicated that a command was not SUCCESS",
        )?
        .add_error(
            "header_mismatch",
            "Number of headers or objects in the response didn't match the number in the request",
        )?
        .add_task_errors()?
        .doc("Result of a command")?
        .build()?;

    let callback = lib.define_future_interface(
        "command_task_callback",
        "Handler for command tasks",
        nothing,
        "Result of the command task",
        command_error,
    )?;

    Ok(callback)
}

fn define_command_builder(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> BackTraced<ClassHandle> {
    let command_set = lib.declare_class("command_set")?;

    let constructor = lib
        .define_constructor(command_set.clone())?
        .doc("Create a new set of commands")?
        .build()?;

    let destructor = lib.define_destructor(command_set.clone(), "Destroy a set of commands")?;

    let finish_header = lib
        .define_method("finish_header", command_set.clone())?

        .doc("Finish any partially completed header. This allows for the construction of two headers with the same type and index")?
        .build()?;

    let add_u8_g12v1 = lib
        .define_method("add_g12_v1_u8", command_set.clone())?
        .param(
            "idx",
            Primitive::U8,
            "Index of the point to send the command to",
        )?
        .param("header", shared.g12v1_struct.clone(), "CROB data")?
        .doc("Add a CROB with 1-byte prefix index")?
        .build()?;

    let add_u16_g12v1 = lib
        .define_method("add_g12_v1_u16", command_set.clone())?
        .param(
            "idx",
            Primitive::U16,
            "Index of the point to send the command to",
        )?
        .param("header", shared.g12v1_struct.clone(), "CROB data")?
        .doc("Add a CROB with 2-byte prefix index")?
        .build()?;

    let add_u8_g41v1 = lib
        .define_method("add_g41_v1_u8", command_set.clone())?
        .param(
            "idx",
            Primitive::U8,
            "Index of the point to send the command to",
        )?
        .param("value", Primitive::S32, "Value to set the analog output to")?
        .doc("Add a Analog Output command (signed 32-bit integer) with 1-byte prefix index")?
        .build()?;

    let add_u16_g41v1 = lib
        .define_method("add_g41_v1_u16", command_set.clone())?
        .param(
            "idx",
            Primitive::U16,
            "Index of the point to send the command to",
        )?
        .param("value", Primitive::S32, "Value to set the analog output to")?
        .doc("Add a Analog Output command (signed 32-bit integer) with 2-byte prefix index")?
        .build()?;

    let add_u8_g41v2 = lib
        .define_method("add_g41_v2_u8", command_set.clone())?
        .param(
            "idx",
            Primitive::U8,
            "Index of the point to send the command to",
        )?
        .param("value", Primitive::S16, "Value to set the analog output to")?
        .doc("Add a Analog Output command (signed 16-bit integer) with 1-byte prefix index")?
        .build()?;

    let add_u16_g41v2 = lib
        .define_method("add_g41_v2_u16", command_set.clone())?
        .param(
            "idx",
            Primitive::U16,
            "Index of the point to send the command to",
        )?
        .param("value", Primitive::S16, "Value to set the analog output to")?
        .doc("Add a Analog Output command (signed 16-bit integer) with 2-byte prefix index")?
        .build()?;

    let add_u8_g41v3 = lib
        .define_method("add_g41_v3_u8", command_set.clone())?
        .param(
            "idx",
            Primitive::U8,
            "Index of the point to send the command to",
        )?
        .param(
            "value",
            Primitive::Float,
            "Value to set the analog output to",
        )?
        .doc("Add a Analog Output command (single-precision float) with 1-byte prefix index")?
        .build()?;

    let add_u16_g41v3 = lib
        .define_method("add_g41_v3_u16", command_set.clone())?
        .param(
            "idx",
            Primitive::U16,
            "Index of the point to send the command to",
        )?
        .param(
            "value",
            Primitive::Float,
            "Value to set the analog output to",
        )?
        .doc("Add a Analog Output command (single-precision float) with 2-byte prefix index")?
        .build()?;

    let add_u8_g41v4 = lib
        .define_method("add_g41_v4_u8", command_set.clone())?
        .param(
            "idx",
            Primitive::U8,
            "Index of the point to send the command to",
        )?
        .param(
            "value",
            Primitive::Double,
            "Value to set the analog output to",
        )?
        .doc("Add a Analog Output command (double-precision float) with 1-byte prefix index")?
        .build()?;

    let add_u16_g41v4 = lib
        .define_method("add_g41_v4_u16", command_set.clone())?
        .param(
            "idx",
            Primitive::U16,
            "Index of the point to send the command to",
        )?
        .param(
            "value",
            Primitive::Double,
            "Value to set the analog output to",
        )?
        .doc("Add a Analog Output command (double-precision float) with 2-byte prefix index")?
        .build()?;

    let command_set = lib
        .define_class(&command_set)?
        .constructor(constructor)?
        .destructor(destructor)?
        .method(add_u8_g12v1)?
        .method(add_u16_g12v1)?
        .method(add_u8_g41v1)?
        .method(add_u16_g41v1)?
        .method(add_u8_g41v2)?
        .method(add_u16_g41v2)?
        .method(add_u8_g41v3)?
        .method(add_u16_g41v3)?
        .method(add_u8_g41v4)?
        .method(add_u16_g41v4)?
        .method(finish_header)?
        .doc("Builder type used to construct command requests")?
        .build()?;

    Ok(command_set)
}

fn define_time_sync_callback(
    lib: &mut LibraryBuilder,
    nothing: EnumHandle,
) -> BackTraced<FutureInterface<Unvalidated>> {
    let time_sync_error = lib
        .define_error_type(
            "time_sync_error",
            "time_sync_exception",
            ExceptionType::CheckedException,
        )?
        .add_error("clock_rollback", "Detected a clock rollback")?
        .add_error(
            "system_time_not_unix",
            "The system time cannot be converted to a Unix timestamp",
        )?
        .add_error(
            "bad_outstation_time_delay",
            "Outstation time delay exceeded the response delay",
        )?
        .add_error("overflow", "Overflow in calculation")?
        .add_error(
            "still_needs_time",
            "Outstation did not clear the NEED_TIME IIN bit",
        )?
        .add_error("system_time_not_available", "System time not available")?
        .add_task_errors()?
        .doc("Possible errors that can occur during a time synchronization procedure")?
        .build()?;

    let callback = lib.define_future_interface(
        "time_sync_task_callback",
        "Handler for time synchronization tasks",
        nothing,
        "Result of the time synchronization task",
        time_sync_error,
    )?;

    Ok(callback)
}

fn define_time_sync_mode(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let mode = lib
        .define_enum("time_sync_mode")?
        .push(
            "lan",
            "Perform a LAN time sync with Record Current Time (0x18) function code",
        )?
        .push(
            "non_lan",
            "Perform a non-LAN time sync with Delay Measurement (0x17) function code",
        )?
        .doc("Time synchronization mode")?
        .build()?;

    Ok(mode)
}

fn define_restart_callback(lib: &mut LibraryBuilder) -> BackTraced<FutureInterface<Unvalidated>> {
    let restart_error = lib
        .define_error_type(
            "restart_error",
            "restart_exception",
            ExceptionType::CheckedException,
        )?
        .add_task_errors()?
        .doc("Errors that can occur during a cold/warm restart operation")?
        .build()?;

    let callback = lib.define_future_interface(
        "restart_task_callback",
        "Handler for restart tasks",
        DurationType::Milliseconds,
        "Result of the restart task",
        restart_error,
    )?;

    Ok(callback)
}

fn define_file_info_callback(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> BackTraced<FutureInterface<Unvalidated>> {
    let callback = lib.define_future_interface(
        "file_info_callback",
        "Callback interface for retrieving file info asynchronously",
        shared.file.file_info.clone(),
        "Information about the requested file",
        shared.file.file_error.clone(),
    )?;

    Ok(callback)
}

fn define_file_reader(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> BackTraced<AsynchronousInterface> {
    let file_reader = lib
        .define_interface(
        "file_reader",
        " Callbacks for reading a file from the outstation asynchronously"
        )?
        // opened
        .begin_callback("opened",
                        doc("Called when the file is successfully opened").details("May optionally abort the operation by returning false"),
        )?
        .param("size", Primitive::U32, "Size of the file returned by the outstation")?
        .returns(Primitive::Bool, "True to continue, false to abort")?
        .end_callback()?
        // block received
        .begin_callback("block_received",
                        doc("Called when the next block is received")
                            .details("May optionally abort the transfer. This allows the application abort on internal errors like being or by user request.")
        )?
        .param("block_num", Primitive::U32, "The block number which increments as the transfer proceeds")?
        .param("data", shared.byte_it.clone(), "{iterator} of bytes in the block")?
        .returns(Primitive::Bool, "True to continue, false to abort")?
        .end_callback()?
        // aborted
        .begin_callback("aborted", "Called when the transfer is aborted before completion due to an error or user request")?
        .param("error", shared.file.file_error.clone_enum(), "Error describing why the transfer aborted")?
        .end_callback()?
        // completed
        .begin_callback("completed", "Called when the transfer completes successfully")?
        .end_callback()?
        .build_async()?;

    Ok(file_reader)
}

fn define_file_read_config(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let config = lib.declare_function_argument_struct("file_read_config")?;

    let max_block_size = Name::create("max_block_size")?;
    let max_file_size = Name::create("max_file_size")?;

    let config = lib
        .define_function_argument_struct(config)?
        .doc("Configuration related to reading a file")?
        .add(
            max_block_size.clone(),
            Primitive::U16,
            "Maximum block size requested by the master during the file open",
        )?
        .add(
            max_file_size.clone(),
            Primitive::U32,
            "Maximum file size accepted by the master",
        )?
        .end_fields()?
        .begin_initializer(
            "defaults",
            InitializerType::Static,
            "Initialize the configuration to default values",
        )?
        .default(&max_block_size, NumberValue::U16(u16::MAX))?
        .default(&max_file_size, NumberValue::U32(u32::MAX))?
        .end_initializer()?
        .build()?;

    Ok(config)
}

fn define_dir_read_config(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let config = lib.declare_function_argument_struct("dir_read_config")?;

    let max_block_size = Name::create("max_block_size")?;
    let max_file_size = Name::create("max_file_size")?;

    let config = lib
        .define_function_argument_struct(config)?
        .doc("Configuration related to reading a file")?
        .add(
            max_block_size.clone(),
            Primitive::U16,
            "Maximum block size requested by the master during the file open",
        )?
        .add(
            max_file_size.clone(),
            Primitive::U32,
            "Maximum number of bytes that may be accumulated while reading directory information",
        )?
        .end_fields()?
        .begin_initializer(
            "defaults",
            InitializerType::Static,
            "Initialize the configuration to default values",
        )?
        .default(&max_block_size, NumberValue::U16(u16::MAX))?
        .default(&max_file_size, NumberValue::U32(2048))?
        .end_initializer()?
        .build()?;

    Ok(config)
}

fn define_link_status_callback(
    lib: &mut LibraryBuilder,
    nothing: EnumHandle,
) -> BackTraced<FutureInterface<Unvalidated>> {
    let link_status_error = lib
        .define_error_type("link_status_error", "link_status_exception", ExceptionType::CheckedException)?
        .add_error(
            "unexpected_response",
            "There was activity on the link, but it wasn't a LINK_STATUS",
        )?
        .add_task_errors()?
        .doc("Errors that can occur during a manually initiated link status check. See {class:master_channel.check_link_status()}")?
        .build()?;

    let callback = lib.define_future_interface(
        "link_status_callback",
        "Handler for link status check",
        nothing,
        "Result of the link status",
        link_status_error,
    )?;

    Ok(callback)
}

fn define_empty_response_callback(
    lib: &mut LibraryBuilder,
    nothing: EnumHandle,
) -> BackTraced<FutureInterface<Unvalidated>> {
    let error = lib
        .define_error_type(
            "empty_response_error",
            "empty_response_exception",
            ExceptionType::CheckedException,
        )?
        .add_error(
            "rejected_by_iin2",
            "IIN2 indicates request was not completely successful",
        )?
        .add_task_errors()?
        .doc("Errors that may occur when performing a request that expects a response with zero object headers")?
        .build()?;

    let callback = lib.define_future_interface(
        "empty_response_callback",
        "Callback interface for any task that expects an empty response",
        nothing,
        "Result of operation",
        error,
    )?;

    Ok(callback)
}
