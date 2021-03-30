#include "dnp3rs.h"

#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
#include <inttypes.h>

// ANCHOR: logging_callback
// callback which will receive log messages
void on_log_message(dnp3_log_level_t level, const char *msg, void *ctx) { printf("%s", msg); }
// ANCHOR_END: logging_callback

// ClientState listener callback
void client_state_on_change(dnp3_client_state_t state, void *arg) { printf("ClientState = %s\n", dnp3_client_state_to_string(state)); }

// ReadHandler callbacks
void begin_fragment(dnp3_read_type_t read_type, dnp3_response_header_t header, void *arg)
{
    printf("Beginning fragment (broadcast: %u)\n", dnp3_iin1_is_set(&header.iin.iin1, DNP3_IIN1_FLAG_BROADCAST));
}

void end_fragment(dnp3_read_type_t read_type, dnp3_response_header_t header, void *arg) { printf("End fragment\n"); }

void handle_binary(dnp3_header_info_t info, dnp3_binary_iterator_t *it, void *arg)
{
    printf("Binaries:\n");
    printf("Qualifier: %s \n", dnp3_qualifier_code_to_string(info.qualifier));
    printf("Variation: %s \n", dnp3_variation_to_string(info.variation));

    dnp3_binary_t *value = NULL;
    while (value = dnp3_binary_next(it)) {
        printf("BI %u: Value=%u Flags=0x%02X Time=%" PRIu64 "\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_double_bit_binary(dnp3_header_info_t info, dnp3_double_bit_binary_iterator_t *it, void *arg)
{
    printf("Double Bit Binaries:\n");
    printf("Qualifier: %s \n", dnp3_qualifier_code_to_string(info.qualifier));
    printf("Variation: %s \n", dnp3_variation_to_string(info.variation));

    dnp3_double_bit_binary_t *value = NULL;
    while (value = dnp3_doublebitbinary_next(it)) {
        printf("DBBI %u: Value=%X Flags=0x%02X Time=%" PRIu64 "\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_binary_output_status(dnp3_header_info_t info, dnp3_binary_output_status_iterator_t *it, void *arg)
{
    printf("Binary Output Statuses:\n");
    printf("Qualifier: %s \n", dnp3_qualifier_code_to_string(info.qualifier));
    printf("Variation: %s \n", dnp3_variation_to_string(info.variation));

    dnp3_binary_output_status_t *value = NULL;
    while (value = dnp3_binaryoutputstatus_next(it)) {
        printf("BOS %u: Value=%u Flags=0x%02X Time=%" PRIu64 "\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_counter(dnp3_header_info_t info, dnp3_counter_iterator_t *it, void *arg)
{
    printf("Counters:\n");
    printf("Qualifier: %s \n", dnp3_qualifier_code_to_string(info.qualifier));
    printf("Variation: %s \n", dnp3_variation_to_string(info.variation));

    dnp3_counter_t *value = NULL;
    while (value = dnp3_counter_next(it)) {
        printf("Counter %u: Value=%u Flags=0x%02X Time=%" PRIu64 "\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_frozen_counter(dnp3_header_info_t info, dnp3_frozen_counter_iterator_t *it, void *arg)
{
    printf("Frozen Counters:\n");
    printf("Qualifier: %s \n", dnp3_qualifier_code_to_string(info.qualifier));
    printf("Variation: %s \n", dnp3_variation_to_string(info.variation));

    dnp3_frozen_counter_t *value = NULL;
    while (value = dnp3_frozencounter_next(it)) {
        printf("Frozen Counter %u: Value=%u Flags=0x%02X Time=%" PRIu64 "\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_analog(dnp3_header_info_t info, dnp3_analog_iterator_t *it, void *arg)
{
    printf("Analogs:\n");
    printf("Qualifier: %s \n", dnp3_qualifier_code_to_string(info.qualifier));
    printf("Variation: %s \n", dnp3_variation_to_string(info.variation));

    dnp3_analog_t *value = NULL;
    while (value = dnp3_analog_next(it)) {
        printf("AI %u: Value=%f Flags=0x%02X Time=%" PRIu64 "\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_analog_output_status(dnp3_header_info_t info, dnp3_analog_output_status_iterator_t *it, void *arg)
{
    printf("Analog Output Statuses:\n");
    printf("Qualifier: %s \n", dnp3_qualifier_code_to_string(info.qualifier));
    printf("Variation: %s \n", dnp3_variation_to_string(info.variation));

    dnp3_analog_output_status_t *value = NULL;
    while (value = dnp3_analogoutputstatus_next(it)) {
        printf("AOS %u: Value=%f Flags=0x%02X Time=%" PRIu64 "\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_octet_strings(dnp3_header_info_t info, dnp3_octet_string_iterator_t *it, void *arg)
{
    printf("Octet Strings:\n");
    printf("Qualifier: %s \n", dnp3_qualifier_code_to_string(info.qualifier));
    printf("Variation: %s \n", dnp3_variation_to_string(info.variation));

    dnp3_octet_string_t *value = NULL;
    while (value = dnp3_octetstring_next(it)) {
        printf("Octet String: %u: Value=", value->index);
        dnp3_byte_t *single_byte = dnp3_byte_next(value->value);
        while (single_byte != NULL) {
            printf("%02X", single_byte->value);
            single_byte = dnp3_byte_next(value->value);
        }

        printf("\n");
    }
}

// Single read callback
void on_read_complete(dnp3_read_result_t result, void *arg) { printf("ReadResult: %s\n", dnp3_read_result_to_string(result)); }

// Command callback
void on_command_complete(dnp3_command_result_t result, void *arg) { printf("CommandResult: %s\n", dnp3_command_result_to_string(result)); }

// Timesync callback
void on_timesync_complete(dnp3_time_sync_result_t result, void *arg) { printf("TimeSyncResult: %s\n", dnp3_time_sync_result_to_string(result)); }

// Restart callback
void on_restart_complete(dnp3_restart_result_t result, void *arg) { printf("RestartResult: %s\n", dnp3_restart_error_to_string(result.error)); }

void on_link_status_complete(dnp3_link_status_result_t result, void *arg) { printf("LinkStatusResult: %s\n", dnp3_link_status_result_to_string(result)); }

// Timestamp callback
dnp3_time_provider_timestamp_t get_time(void *arg)
{
    time_t timer = time(NULL);

    return dnp3_timeprovidertimestamp_valid(timer * 1000);
}

int main()
{
    // ANCHOR: logging_init
    // define logger callback "interface"
    dnp3_logger_t logger = {
        // function pointer where log messages will be sent
        .on_message = &on_log_message,
        // no context to free
        .on_destroy = NULL,
        // optional context argument applied to all log callbacks
        .ctx = NULL,
    };

    // initialize logging with the default configuration
    dnp3_configure_logging(dnp3_logging_config_init(), logger);
    // ANCHOR_END: logging_init

    // long-lived types that must be freed before exit
    // ANCHOR: runtime_declare
    dnp3_runtime_t* runtime = NULL;
    // ANCHOR_END: runtime_declare
    dnp3_master_t* master = NULL;

    // ANCHOR: runtime_init
    // create the runtime
    dnp3_runtime_config_t runtime_config = {
        .num_core_threads = 4,
    };    
    if (dnp3_runtime_new(runtime_config, &runtime)) {

        goto cleanup;
    }        
    // ANCHOR_END: runtime_init

    // Create the master
    dnp3_master_config_t master_config = dnp3_master_config_init(1);
    master_config.decode_level.application = DNP3_APP_DECODE_LEVEL_OBJECT_VALUES;

    dnp3_endpoint_list_t *endpoints = dnp3_endpoint_list_new("127.0.0.1:20000");
    dnp3_client_state_listener_t listener = {
        .on_change = &client_state_on_change,
        .on_destroy = NULL,
        .ctx = NULL,
    };
    
    if (dnp3_master_create_tcp_session(runtime, DNP3_LINK_ERROR_MODE_CLOSE, master_config, endpoints, dnp3_retry_strategy_init(), 1000, listener, &master)) {
        goto cleanup;
    }

    dnp3_endpoint_list_destroy(endpoints);

    // Create the association
    dnp3_read_handler_t read_handler = {
        .begin_fragment = &begin_fragment,
        .end_fragment = &end_fragment,
        .handle_binary = &handle_binary,
        .handle_double_bit_binary = &handle_double_bit_binary,
        .handle_binary_output_status = &handle_binary_output_status,
        .handle_counter = &handle_counter,
        .handle_frozen_counter = &handle_frozen_counter,
        .handle_analog = &handle_analog,
        .handle_analog_output_status = &handle_analog_output_status,
        .handle_octet_string = &handle_octet_strings,
        .on_destroy = NULL,
        .ctx = NULL,
    };

    dnp3_association_config_t association_config = dnp3_association_config_init(
        // disable unsolicited (Class 1/2/3)
        dnp3_event_classes_all(),
        // after the integrity poll, enable unsolicited (Class 1/2/3)
        dnp3_event_classes_all(),
        // perform an integrity poll with Class 1/2/3/0
        dnp3_classes_all(),
        // don't automatically scan Class 1/2/3 when the corresponding IIN bit is asserted
        dnp3_event_classes_none());
    association_config.auto_time_sync = DNP3_AUTO_TIME_SYNC_LAN;
    association_config.keep_alive_timeout = 60;

    dnp3_time_provider_t time_provider = {
        .get_time = get_time,
        .on_destroy = NULL,
        .ctx = NULL,
    };
    dnp3_association_id_t association_id;
    if (dnp3_master_add_association(master, 1024, association_config, read_handler, time_provider, &association_id)) {
        goto cleanup;
    }

    // Add an event poll
    dnp3_request_t *poll_request = dnp3_request_new_class(false, true, true, true);
    dnp3_poll_id_t poll_id;
    dnp3_master_add_poll(master, association_id, poll_request, 5000, &poll_id);
    dnp3_request_destroy(poll_request);

    // start communications
    dnp3_master_enable(master);

    char cbuf[10];
    while (true) {
        fgets(cbuf, 10, stdin);

        if (strcmp(cbuf, "x\n") == 0) {
            goto cleanup;
        }
        else if (strcmp(cbuf, "enable\n") == 0) {
            printf("calling enable\n");
            dnp3_master_enable(master);
        }
        else if (strcmp(cbuf, "disable\n") == 0) {
            printf("calling disable\n");
            dnp3_master_disable(master);
        }
        else if (strcmp(cbuf, "dln\n") == 0) {
            dnp3_master_set_decode_level(master, dnp3_decode_level_init());
        }
        else if (strcmp(cbuf, "dlv\n") == 0) {
            dnp3_decode_level_t level = dnp3_decode_level_init();
            level.application = DNP3_APP_DECODE_LEVEL_OBJECT_VALUES;
            dnp3_master_set_decode_level(master, level);
        }
        else if (strcmp(cbuf, "rao\n") == 0) {
            dnp3_request_t *request = dnp3_request_new();
            dnp3_request_add_all_objects_header(request, DNP3_VARIATION_GROUP40_VAR0);

            dnp3_read_task_callback_t cb = {
                .on_complete = &on_read_complete,
                .on_destroy = NULL,
                .ctx = NULL,
            };
            dnp3_master_read(master, association_id, request, cb);

            dnp3_request_destroy(request);
        }
        else if (strcmp(cbuf, "rmo\n") == 0) {
            dnp3_request_t *request = dnp3_request_new();
            dnp3_request_add_all_objects_header(request, DNP3_VARIATION_GROUP10_VAR0);
            dnp3_request_add_all_objects_header(request, DNP3_VARIATION_GROUP40_VAR0);

            dnp3_read_task_callback_t cb = {
                .on_complete = &on_read_complete,
                .on_destroy = NULL,
                .ctx = NULL,
            };
            dnp3_master_read(master, association_id, request, cb);

            dnp3_request_destroy(request);
        }
        else if (strcmp(cbuf, "cmd\n") == 0) {
            dnp3_commands_t *commands = dnp3_commands_new();
            dnp3_g12v1_t g12v1 = dnp3_g12v1_init(dnp3_control_code_init(DNP3_TRIP_CLOSE_CODE_NUL, false, DNP3_OP_TYPE_LATCH_ON), 1, 1000, 1000);
            dnp3_commands_add_g12v1_u16(commands, 3, g12v1);

            dnp3_command_task_callback_t cb = {
                .on_complete = &on_command_complete,
                .on_destroy = NULL,
                .ctx = NULL,
            };

            dnp3_master_operate(master, association_id, DNP3_COMMAND_MODE_SELECT_BEFORE_OPERATE, commands, cb);

            dnp3_commands_destroy(commands);
        }
        else if (strcmp(cbuf, "evt\n") == 0) {
            dnp3_master_demand_poll(master, poll_id);
        }
        else if (strcmp(cbuf, "lts\n") == 0) {
            dnp3_time_sync_task_callback_t cb = {
                .on_complete = &on_timesync_complete,
                .on_destroy = NULL,
                .ctx = NULL,
            };
            dnp3_master_sync_time(master, association_id, DNP3_TIME_SYNC_MODE_LAN, cb);
        }
        else if (strcmp(cbuf, "nts\n") == 0) {
            dnp3_time_sync_task_callback_t cb = {
                .on_complete = &on_timesync_complete,
                .on_destroy = NULL,
                .ctx = NULL,
            };
            dnp3_master_sync_time(master, association_id, DNP3_TIME_SYNC_MODE_NON_LAN, cb);
        }
        else if (strcmp(cbuf, "crt\n") == 0) {
            dnp3_restart_task_callback_t cb = {
                .on_complete = &on_restart_complete,
                .on_destroy = NULL,
                .ctx = NULL,
            };
            dnp3_master_cold_restart(master, association_id, cb);
        }
        else if (strcmp(cbuf, "wrt\n") == 0) {
            dnp3_restart_task_callback_t cb = {
                .on_complete = &on_restart_complete,
                .on_destroy = NULL,
                .ctx = NULL,
            };
            dnp3_master_warm_restart(master, association_id, cb);
        }
        else if (strcmp(cbuf, "lsr\n") == 0) {
            dnp3_link_status_callback_t cb = {
                .on_complete = &on_link_status_complete,
                .on_destroy = NULL,
                .ctx = NULL,
            };
            dnp3_master_check_link_status(master, association_id, cb);
        }
        else {
            printf("Unknown command\n");
        }
    }
    
// all of the destroy functions are NULL-safe
cleanup:    
    dnp3_master_destroy(master);
    // ANCHOR: runtime_destroy
    dnp3_runtime_destroy(runtime);
    // ANCHOR_END: runtime_destroy

    return 0;
}
