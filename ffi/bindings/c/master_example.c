#include "dnp3.h"

#include <inttypes.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
#include <stdlib.h>

// ANCHOR: logging_callback
// callback which will receive log messages
void on_log_message(dnp3_log_level_t level, const char *msg, void *ctx) { printf("%s", msg); }

dnp3_logger_t get_logger()
{
    return (dnp3_logger_t){
        // function pointer where log messages will be sent
        .on_message = &on_log_message,
        // no context to free
        .on_destroy = NULL,
        // optional context argument applied to all log callbacks
        .ctx = NULL,
    };
}
// ANCHOR_END: logging_callback

// ClientState listener callback
void client_state_on_change(dnp3_client_state_t state, void *arg) { printf("ClientState = %s\n", dnp3_client_state_to_string(state)); }

dnp3_client_state_listener_t get_client_state_listener()
{
    return (dnp3_client_state_listener_t){
        .on_change = &client_state_on_change,
        .on_destroy = NULL,
        .ctx = NULL,
    };
}

// PortState listener callback
void port_state_on_change(dnp3_port_state_t state, void *arg) { printf("PortState = %s\n", dnp3_port_state_to_string(state)); }

dnp3_port_state_listener_t get_port_state_listener()
{    
    return (dnp3_port_state_listener_t){
        .on_change = &port_state_on_change,
        .on_destroy = NULL,
        .ctx = NULL,
    };
}

// ANCHOR: read_handler
void begin_fragment(dnp3_read_type_t read_type, dnp3_response_header_t header, void *arg)
{
    printf("Beginning fragment (broadcast: %u)\n", header.iin.iin1.broadcast);
}

void end_fragment(dnp3_read_type_t read_type, dnp3_response_header_t header, void *arg) { printf("End fragment\n"); }

void handle_binary_input(dnp3_header_info_t info, dnp3_binary_input_iterator_t *it, void *arg)
{
    printf("Binaries:\n");
    printf("Qualifier: %s \n", dnp3_qualifier_code_to_string(info.qualifier));
    printf("Variation: %s \n", dnp3_variation_to_string(info.variation));

    dnp3_binary_input_t *value = NULL;
    while ((value = dnp3_binary_input_iterator_next(it))) {
        printf("BI %u: Value=%u Flags=0x%02X Time=%" PRIu64 "\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_double_bit_binary_input(dnp3_header_info_t info, dnp3_double_bit_binary_input_iterator_t *it, void *arg)
{
    printf("Double Bit Binaries:\n");
    printf("Qualifier: %s \n", dnp3_qualifier_code_to_string(info.qualifier));
    printf("Variation: %s \n", dnp3_variation_to_string(info.variation));

    dnp3_double_bit_binary_input_t *value = NULL;
    while ((value = dnp3_double_bit_binary_input_iterator_next(it))) {
        printf("DBBI %u: Value=%X Flags=0x%02X Time=%" PRIu64 "\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_binary_output_status(dnp3_header_info_t info, dnp3_binary_output_status_iterator_t *it, void *arg)
{
    printf("Binary Output Statuses:\n");
    printf("Qualifier: %s \n", dnp3_qualifier_code_to_string(info.qualifier));
    printf("Variation: %s \n", dnp3_variation_to_string(info.variation));

    dnp3_binary_output_status_t *value = NULL;
    while ((value = dnp3_binary_output_status_iterator_next(it))) {
        printf("BOS %u: Value=%u Flags=0x%02X Time=%" PRIu64 "\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_counter(dnp3_header_info_t info, dnp3_counter_iterator_t *it, void *arg)
{
    printf("Counters:\n");
    printf("Qualifier: %s \n", dnp3_qualifier_code_to_string(info.qualifier));
    printf("Variation: %s \n", dnp3_variation_to_string(info.variation));

    dnp3_counter_t *value = NULL;
    while ((value = dnp3_counter_iterator_next(it))) {
        printf("Counter %u: Value=%u Flags=0x%02X Time=%" PRIu64 "\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_frozen_counter(dnp3_header_info_t info, dnp3_frozen_counter_iterator_t *it, void *arg)
{
    printf("Frozen Counters:\n");
    printf("Qualifier: %s \n", dnp3_qualifier_code_to_string(info.qualifier));
    printf("Variation: %s \n", dnp3_variation_to_string(info.variation));

    dnp3_frozen_counter_t *value = NULL;
    while ((value = dnp3_frozen_counter_iterator_next(it))) {
        printf("Frozen Counter %u: Value=%u Flags=0x%02X Time=%" PRIu64 "\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_analog_input(dnp3_header_info_t info, dnp3_analog_input_iterator_t *it, void *arg)
{
    printf("Analogs:\n");
    printf("Qualifier: %s \n", dnp3_qualifier_code_to_string(info.qualifier));
    printf("Variation: %s \n", dnp3_variation_to_string(info.variation));

    dnp3_analog_input_t *value = NULL;
    while ((value = dnp3_analog_input_iterator_next(it))) {
        printf("AI %u: Value=%f Flags=0x%02X Time=%" PRIu64 "\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_analog_output_status(dnp3_header_info_t info, dnp3_analog_output_status_iterator_t *it, void *arg)
{
    printf("Analog Output Statuses:\n");
    printf("Qualifier: %s \n", dnp3_qualifier_code_to_string(info.qualifier));
    printf("Variation: %s \n", dnp3_variation_to_string(info.variation));

    dnp3_analog_output_status_t *value = NULL;
    while ((value = dnp3_analog_output_status_iterator_next(it))) {
        printf("AOS %u: Value=%f Flags=0x%02X Time=%" PRIu64 "\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_octet_strings(dnp3_header_info_t info, dnp3_octet_string_iterator_t *it, void *arg)
{
    printf("Octet Strings:\n");
    printf("Qualifier: %s \n", dnp3_qualifier_code_to_string(info.qualifier));
    printf("Variation: %s \n", dnp3_variation_to_string(info.variation));

    dnp3_octet_string_t *value = NULL;
    while ((value = dnp3_octet_string_iterator_next(it))) {
        printf("Octet String: %u: Value=", value->index);
        uint8_t *byte = dnp3_byte_iterator_next(value->value);
        while (byte != NULL) {
            printf("%02X", *byte);
            byte = dnp3_byte_iterator_next(value->value);
        }

        printf("\n");
    }
}

void handle_string_attr(dnp3_header_info_t info, dnp3_string_attr_t attr, uint8_t set, uint8_t variation, const char* value, void *arg)
{
    printf("String attribute: %s set: %d var: %d value: %s \n", dnp3_string_attr_to_string(attr), set, variation, value);
}
// ANCHOR_END: read_handler

dnp3_read_handler_t get_read_handler()
{
    return (dnp3_read_handler_t){
        .begin_fragment = &begin_fragment,
        .end_fragment = &end_fragment,
        .handle_binary_input = &handle_binary_input,
        .handle_double_bit_binary_input = &handle_double_bit_binary_input,
        .handle_binary_output_status = &handle_binary_output_status,
        .handle_counter = &handle_counter,
        .handle_frozen_counter = &handle_frozen_counter,
        .handle_analog_input = &handle_analog_input,
        .handle_analog_output_status = &handle_analog_output_status,
        .handle_octet_string = &handle_octet_strings,
        .handle_string_attr = &handle_string_attr,        
        .on_destroy = NULL,
        .ctx = NULL,
    };
}

// read callbacks
void on_read_success(dnp3_nothing_t nothing, void *arg) { printf("read success! \n"); }
void on_read_failure(dnp3_read_error_t error, void* arg) { printf("read error: %s \n", dnp3_read_error_to_string(error)); }

// Command callbacks
// ANCHOR: assoc_control_callback
void on_command_success(dnp3_nothing_t nothing, void* arg)
{
    printf("command success!\n");
}
void on_command_error(dnp3_command_error_t result, void *arg)
{
    printf("command failed: %s\n", dnp3_command_error_to_string(result));
}
// ANCHOR_END: assoc_control_callback

// time sync callbacks
void on_time_sync_success(dnp3_nothing_t nothing, void* arg) { printf("time sync success! \n"); }
void on_time_sync_error(dnp3_time_sync_error_t error, void *arg) { printf("Time sync error: %s\n", dnp3_time_sync_error_to_string(error)); }

// warm/cold restart callbacks
void on_restart_success(uint64_t delay, void *arg) { printf("restart success: %" PRIu64 "\n", delay); }
void on_restart_failure(dnp3_restart_error_t error, void* arg) { printf("Restart failure: %s\n", dnp3_restart_error_to_string(error)); }

// link status callbacks
void on_link_status_success(dnp3_nothing_t nothing, void *arg) { printf("link status success!\n"); }
void on_link_status_failure(dnp3_link_status_error_t error, void* arg) { printf("link status error: %s\n", dnp3_link_status_error_to_string(error)); }

// generic callbacks
void on_generic_success(dnp3_nothing_t delay, void *arg) { printf("%s success! \n", (const char*) arg); }
void on_generic_failure(dnp3_empty_response_error_t error, void *arg)
{
    printf("%s failure: %s\n", (const char *)arg, dnp3_empty_response_error_to_string(error));
}

// ANCHOR: master_channel_config
dnp3_master_channel_config_t get_master_channel_config()
{
    dnp3_master_channel_config_t config = dnp3_master_channel_config_init(1);
    config.decode_level.application = DNP3_APP_DECODE_LEVEL_OBJECT_VALUES;
    return config;
}
// ANCHOR_END: master_channel_config

// ANCHOR: association_config
dnp3_association_config_t get_association_config()
{
    dnp3_association_config_t config = dnp3_association_config_init(
        // disable unsolicited first (Class 1/2/3)
        dnp3_event_classes_all(),
        // after the integrity poll, enable unsolicited (Class 1/2/3)
        dnp3_event_classes_all(),
        // perform startup integrity poll with Class 1/2/3/0
        dnp3_classes_all(),
        // don't automatically scan Class 1/2/3 when the corresponding IIN bit is asserted
        dnp3_event_classes_none());

    config.auto_time_sync = DNP3_AUTO_TIME_SYNC_LAN;
    config.keep_alive_timeout = 60;
    return config;
}
// ANCHOR_END: association_config

// ANCHOR: association_handler
dnp3_utc_timestamp_t get_system_time(void *arg)
{
    time_t timer = time(NULL);

    return dnp3_utc_timestamp_valid(timer * 1000);
}

dnp3_association_handler_t get_association_handler()
{
    return (dnp3_association_handler_t){
        .get_current_time = get_system_time,
        .on_destroy = NULL,
        .ctx = NULL,
    };
}
// ANCHOR_END: association_handler

// ANCHOR: association_information
void task_start(dnp3_task_type_t task_type, dnp3_function_code_t fc, uint8_t seq, void *arg)
{

}

void task_success(dnp3_task_type_t task_type, dnp3_function_code_t fc, uint8_t seq, void *arg)
{
    
}

void task_fail(dnp3_task_type_t task_type, dnp3_task_error_t error, void *arg)
{
    
}

void unsolicited_response(bool is_duplicate, uint8_t seq, void *arg)
{
    
}

dnp3_association_information_t get_association_information()
{
    return (dnp3_association_information_t){
        .task_start = task_start,
        .task_success = task_success,
        .task_fail = task_fail,
        .unsolicited_response = unsolicited_response,
        .on_destroy = NULL,
        .ctx = NULL,
    };
}
// ANCHOR_END: association_information

int run_channel(dnp3_master_channel_t *channel)
{
    // Create the association
    // ANCHOR: association_create
    dnp3_association_id_t association_id;
    dnp3_param_error_t err =
        dnp3_master_channel_add_association(
            channel,
            1024,
            get_association_config(),
            get_read_handler(),
            get_association_handler(),
            get_association_information(),
            &association_id
        );
    // ANCHOR_END: association_create
    if (err) {
        printf("unable to add association: %s \n", dnp3_param_error_to_string(err));
        return -1;
    }

    // Add an event poll
    // ANCHOR: add_poll
    dnp3_request_t *poll_request = dnp3_request_new_class(false, true, true, true);
    dnp3_poll_id_t poll_id;
    err = dnp3_master_channel_add_poll(channel, association_id, poll_request, 5000, &poll_id);
    dnp3_request_destroy(poll_request);
    // ANCHOR_END: add_poll
    if (err) {
        printf("unable to add poll: %s \n", dnp3_param_error_to_string(err));
        return -1;
    }

    // start communications
    err = dnp3_master_channel_enable(channel);
    if (err) {
        printf("unable to start channel: %s \n", dnp3_param_error_to_string(err));
        return -1;
    }

    char cbuf[10];
    while (true) {
        fgets(cbuf, 10, stdin);

        if (strcmp(cbuf, "x\n") == 0) {
            break;
        }
        else if (strcmp(cbuf, "enable\n") == 0) {
            printf("calling enable\n");
            dnp3_master_channel_enable(channel);
        }
        else if (strcmp(cbuf, "disable\n") == 0) {
            printf("calling disable\n");
            dnp3_master_channel_disable(channel);
        }
        else if (strcmp(cbuf, "dln\n") == 0) {
            dnp3_master_channel_set_decode_level(channel, dnp3_decode_level_nothing());
        }
        else if (strcmp(cbuf, "dlv\n") == 0) {
            dnp3_decode_level_t level = dnp3_decode_level_nothing();
            level.application = DNP3_APP_DECODE_LEVEL_OBJECT_VALUES;
            dnp3_master_channel_set_decode_level(channel, level);
        }
        else if (strcmp(cbuf, "rao\n") == 0) {
            dnp3_request_t *request = dnp3_request_create();
            dnp3_request_add_all_objects_header(request, DNP3_VARIATION_GROUP40_VAR0);

            dnp3_read_task_callback_t cb = {
                .on_complete = &on_read_success,
                .on_failure = &on_read_failure,
                .on_destroy = NULL,
                .ctx = NULL,
            };
            dnp3_master_channel_read(channel, association_id, request, cb);

            dnp3_request_destroy(request);
        }
        else if (strcmp(cbuf, "rmo\n") == 0) {
            dnp3_request_t *request = dnp3_request_create();
            dnp3_request_add_all_objects_header(request, DNP3_VARIATION_GROUP10_VAR0);
            dnp3_request_add_all_objects_header(request, DNP3_VARIATION_GROUP40_VAR0);

            dnp3_read_task_callback_t cb = {
                .on_complete = &on_read_success,
                .on_failure = &on_read_failure,
                .on_destroy = NULL,
                .ctx = NULL,
            };
            dnp3_master_channel_read(channel, association_id, request, cb);

            dnp3_request_destroy(request);
        }
        else if (strcmp(cbuf, "cmd\n") == 0) {
            // ANCHOR: assoc_control
            dnp3_command_set_t *commands = dnp3_command_set_create();
            dnp3_group12_var1_t g12v1 = dnp3_group12_var1_init(dnp3_control_code_init(DNP3_TRIP_CLOSE_CODE_NUL, false, DNP3_OP_TYPE_LATCH_ON), 1, 1000, 1000);
            dnp3_command_set_add_g12_v1_u16(commands, 3, g12v1);

            dnp3_command_task_callback_t cb = {
                .on_complete = &on_command_success,
                .on_failure = &on_command_error,
                .on_destroy = NULL,
                .ctx = NULL,
            };

            dnp3_master_channel_operate(channel, association_id, DNP3_COMMAND_MODE_SELECT_BEFORE_OPERATE, commands, cb);

            dnp3_command_set_destroy(commands);
            // ANCHOR_END: assoc_control
        }
        else if (strcmp(cbuf, "evt\n") == 0) {
            dnp3_master_channel_demand_poll(channel, poll_id);
        }
        else if (strcmp(cbuf, "lts\n") == 0) {
            dnp3_time_sync_task_callback_t cb = {
                .on_complete = &on_time_sync_success,
                .on_failure = &on_time_sync_error,
                .on_destroy = NULL,
                .ctx = NULL,
            };
            dnp3_master_channel_synchronize_time(channel, association_id, DNP3_TIME_SYNC_MODE_LAN, cb);
        }
        else if (strcmp(cbuf, "nts\n") == 0) {
            dnp3_time_sync_task_callback_t cb = {
                .on_complete = &on_time_sync_success,
                .on_failure = &on_time_sync_error,
                .on_destroy = NULL,
                .ctx = NULL,
            };
            dnp3_master_channel_synchronize_time(channel, association_id, DNP3_TIME_SYNC_MODE_NON_LAN, cb);
        }
        else if (strcmp(cbuf, "wad\n") == 0) {
            // ANCHOR: write_dead_bands
            dnp3_empty_response_callback_t cb = {
                .on_complete = &on_generic_success,
                .on_failure = &on_generic_failure,
                .on_destroy = NULL,
                .ctx = "write dead-bands",
            };

            dnp3_write_dead_band_request_t *request = dnp3_write_dead_band_request_create();
            dnp3_write_dead_band_request_add_g34v1_u8(request, 3, 5);
            dnp3_write_dead_band_request_add_g34v3_u16(request, 4, 2.5f);

            dnp3_master_channel_write_dead_bands(channel, association_id, request, cb);
            dnp3_write_dead_band_request_destroy(request);
            // ANCHOR_END: write_dead_bands
        }
        else if (strcmp(cbuf, "fat\n") == 0) {            
            dnp3_empty_response_callback_t cb = {
                .on_complete = &on_generic_success,
                .on_failure = &on_generic_failure,
                .on_destroy = NULL,
                .ctx = "freeze-at-time",
            };

            dnp3_request_t *request = dnp3_request_create();
            dnp3_request_add_time_and_interval(request, 0xFF0000000000, 865000000);
            dnp3_request_add_all_objects_header(request, DNP3_VARIATION_GROUP20_VAR0);
                     
            
            dnp3_master_channel_send_and_expect_empty_response(channel, association_id, DNP3_FUNCTION_CODE_FREEZE_AT_TIME, request, cb);
            dnp3_request_destroy(request);            
        }
        else if (strcmp(cbuf, "rda\n") == 0) {
            // ANCHOR: read_attributes
            dnp3_request_t *request = dnp3_request_create();
            dnp3_request_add_specific_attribute(request, DNP3_ATTRIBUTE_VARIATIONS_ALL_ATTRIBUTES_REQUEST, 0);

            dnp3_read_task_callback_t cb = {
                .on_complete = &on_read_success,
                .on_failure = &on_read_failure,
                .on_destroy = NULL,
                .ctx = NULL,
            };

            dnp3_master_channel_read(channel, association_id, request, cb);
            dnp3_request_destroy(request);
            // ANCHOR_END: read_attributes
        }
        else if (strcmp(cbuf, "wda\n") == 0) {
            // ANCHOR: write_attribute
            dnp3_request_t *request = dnp3_request_create();
            dnp3_request_add_string_attribute(request, DNP3_ATTRIBUTE_VARIATIONS_USER_ASSIGNED_LOCATION, 0, "Mt. Olympus");

            dnp3_empty_response_callback_t cb = {
                .on_complete = &on_generic_success,
                .on_failure = &on_generic_failure,
                .on_destroy = NULL,
                .ctx = "write device attribute",
            };
            dnp3_master_channel_send_and_expect_empty_response(channel, association_id, DNP3_FUNCTION_CODE_WRITE, request, cb);
            dnp3_request_destroy(request);
            // ANCHOR_END: write_attribute
        }
        else if (strcmp(cbuf, "ral\n") == 0) {
            dnp3_request_t *request = dnp3_request_create();
            dnp3_request_add_specific_attribute(request, DNP3_ATTRIBUTE_VARIATIONS_LIST_OF_VARIATIONS, 0);

            dnp3_read_task_callback_t cb = {
                .on_complete = &on_read_success,
                .on_failure = &on_read_failure,
                .on_destroy = NULL,
                .ctx = NULL,
            };
            dnp3_master_channel_read(channel, association_id, request, cb);

            dnp3_request_destroy(request);
        }
        else if (strcmp(cbuf, "crt\n") == 0) {
            dnp3_restart_task_callback_t cb = {
                .on_complete = &on_restart_success,
                .on_failure = &on_restart_failure,
                .on_destroy = NULL,
                .ctx = NULL,
            };
            dnp3_master_channel_cold_restart(channel, association_id, cb);
        }        
        else if (strcmp(cbuf, "wrt\n") == 0) {
            dnp3_restart_task_callback_t cb = {
                .on_complete = &on_restart_success,
                .on_failure = &on_restart_failure,
                .on_destroy = NULL,
                .ctx = NULL,
            };
            dnp3_master_channel_warm_restart(channel, association_id, cb);
        }
        else if (strcmp(cbuf, "lsr\n") == 0) {
            dnp3_link_status_callback_t cb = {
                .on_complete = &on_link_status_success,
                .on_failure = &on_link_status_failure,
                .on_destroy = NULL,
                .ctx = NULL,
            };
            dnp3_master_channel_check_link_status(channel, association_id, cb);
        }
        else {
            printf("Unknown command\n");
        }
    }

    dnp3_master_channel_destroy(channel);
    return 0;
}

int run_tcp_channel(dnp3_runtime_t *runtime)
{
    // ANCHOR: create_master_tcp_channel
    dnp3_master_channel_t* channel = NULL;
    dnp3_endpoint_list_t* endpoints = dnp3_endpoint_list_create("127.0.0.1:20000");
    dnp3_param_error_t err = dnp3_master_channel_create_tcp(
        runtime,
        DNP3_LINK_ERROR_MODE_CLOSE,
        get_master_channel_config(),
        endpoints,
        dnp3_connect_strategy_init(),
        get_client_state_listener(),
        &channel
    );
    dnp3_endpoint_list_destroy(endpoints);
    // ANCHOR_END: create_master_tcp_channel

    if (err) {
        printf("unable to create TCP channel: %s \n", dnp3_param_error_to_string(err));
        return -1;
    }

    return run_channel(channel);
}

int run_serial_channel(dnp3_runtime_t *runtime)
{    
    // ANCHOR: create_master_serial_channel
    dnp3_master_channel_t *channel = NULL;
    dnp3_param_error_t err = dnp3_master_channel_create_serial(
        runtime,
        get_master_channel_config(),
        "/dev/pts/4",
        dnp3_serial_settings_init(),
        1000,
        get_port_state_listener(),
        &channel
    );               
    // ANCHOR_END: create_master_serial_channel

    if (err) {
        printf("unable to create serial channel: %s \n", dnp3_param_error_to_string(err));
        return -1;
    }

    return run_channel(channel);
}

int run_tls_channel(dnp3_runtime_t *runtime, dnp3_tls_client_config_t tls_config)
{
    // ANCHOR: create_master_tls_channel
    dnp3_master_channel_t *channel = NULL;
    dnp3_endpoint_list_t *endpoints = dnp3_endpoint_list_create("127.0.0.1:20001");
    dnp3_param_error_t err = dnp3_master_channel_create_tls(
        runtime,
        DNP3_LINK_ERROR_MODE_CLOSE,
        get_master_channel_config(),
        endpoints,
        dnp3_connect_strategy_init(),
        get_client_state_listener(),
        tls_config,
        &channel
    );
    dnp3_endpoint_list_destroy(endpoints);
    // ANCHOR_END: create_master_tls_channel
    
    if (err) {
        printf("unable to create TLS channel: %s \n", dnp3_param_error_to_string(err));
        return -1;
    }

    return run_channel(channel);
}

dnp3_tls_client_config_t get_ca_tls_config()
{   
    // ANCHOR: tls_ca_chain_config
    dnp3_tls_client_config_t config = dnp3_tls_client_config_init(
        "test.com", 
        "./certs/ca_chain/ca_cert.pem",
        "./certs/ca_chain/entity1_cert.pem",
        "./certs/ca_chain/entity1_key.pem",
        "" // no password
    );
    // ANCHOR_END: tls_ca_chain_config

    return config;
}

dnp3_tls_client_config_t get_self_signed_tls_config()
{
    // ANCHOR: tls_self_signed_config
    dnp3_tls_client_config_t config = dnp3_tls_client_config_init(
        "test.com", 
        "./certs/self_signed/entity2_cert.pem",
        "./certs/self_signed/entity1_cert.pem",
        "./certs/self_signed/entity1_key.pem",
        "" // no password
    );

    config.certificate_mode = DNP3_CERTIFICATE_MODE_SELF_SIGNED;
    // ANCHOR_END: tls_self_signed_config

    return config;
}

// create a channel based on the command line arguments
dnp3_param_error_t create_and_run_channel(int argc, char *argv[], dnp3_runtime_t *runtime)
{
    if(argc != 2) {
        printf("you must specify a transport type\n");
        printf("usage: master-example <channel> (tcp, serial, tls-ca, tls-self-signed)\n");
        return -1;
    }

    if (strcmp(argv[1], "tcp") == 0) {
        return run_tcp_channel(runtime);
    }
    else if (strcmp(argv[1], "serial") == 0) {
        return run_serial_channel(runtime);
    }
    else if (strcmp(argv[1], "tls-ca") == 0) {
        return run_tls_channel(runtime, get_ca_tls_config());
    }
    else if (strcmp(argv[1], "tls-self-signed") == 0) {
        return run_tls_channel(runtime, get_self_signed_tls_config());
    }
    else {
        printf("unknown channel type: %s\n", argv[1]);
        return -1;
    }
}

int main(int argc, char *argv[])
{
    // ANCHOR: logging_init
    // initialize logging with the default configuration
    dnp3_configure_logging(dnp3_logging_config_init(), get_logger());
    // ANCHOR_END: logging_init

    // ANCHOR: runtime_create
    dnp3_runtime_t *runtime = NULL;
    dnp3_runtime_config_t runtime_config = dnp3_runtime_config_init();
    runtime_config.num_core_threads = 4;
    dnp3_param_error_t err = dnp3_runtime_create(runtime_config, &runtime);
    // ANCHOR_END: runtime_create

    if (err) {
        printf("unable to create runtime: %s \n", dnp3_param_error_to_string(err));
        return -1;
    }

    // create a channel based on the cmd line arguments and run it
    int res = create_and_run_channel(argc, argv, runtime);
    
    // ANCHOR: runtime_destroy
    dnp3_runtime_destroy(runtime);
    // ANCHOR_END: runtime_destroy

    return res;
}
