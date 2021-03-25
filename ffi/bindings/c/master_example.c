#include "dnp3rs.h"

#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <time.h>

void print_qualifier(qualifier_code_t qualifier) { printf(Variation_to_string(qualifier)); }

void print_variation(variation_t variation) { printf(Variation_to_string(variation)); }

// ANCHOR: logging_callback
// callback which will receive log messages
void on_log_message(log_level_t level, const char *msg, void *ctx) { printf("%s", msg); }
// ANCHOR_END: logging_callback

// ClientState listener callback
void client_state_on_change(client_state_t state, void *arg) { printf("ClientState = %s\n", ClientState_to_string(state)); }

// ReadHandler callbacks
void begin_fragment(read_type_t read_type, response_header_t header, void *arg)
{
    printf("Beginning fragment (broadcast: %u)\n", iin1_is_set(&header.iin.iin1, Iin1Flag_Broadcast));
}

void end_fragment(read_type_t read_type, response_header_t header, void *arg) { printf("End fragment\n"); }

void handle_binary(header_info_t info, binary_iterator_t *it, void *arg)
{
    printf("Binaries:\n");
    printf("Qualifier: ");
    print_qualifier(info.qualifier);
    printf("\n");
    printf("Variation: ");
    print_variation(info.variation);
    printf("\n");

    binary_t *value = NULL;
    while (value = binary_next(it)) {
        printf("BI %u: Value=%u Flags=0x%02X Time=%llu\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_double_bit_binary(header_info_t info, double_bit_binary_iterator_t *it, void *arg)
{
    printf("Double Bit Binaries:\n");
    printf("Qualifier: ");
    print_qualifier(info.qualifier);
    printf("\n");
    printf("Variation: ");
    print_variation(info.variation);
    printf("\n");

    double_bit_binary_t *value = NULL;
    while (value = doublebitbinary_next(it)) {
        printf("DBBI %u: Value=%X Flags=0x%02X Time=%llu\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_binary_output_status(header_info_t info, binary_output_status_iterator_t *it, void *arg)
{
    printf("Binary Output Statuses:\n");
    printf("Qualifier: ");
    print_qualifier(info.qualifier);
    printf("\n");
    printf("Variation: ");
    print_variation(info.variation);
    printf("\n");

    binary_output_status_t *value = NULL;
    while (value = binaryoutputstatus_next(it)) {
        printf("BOS %u: Value=%u Flags=0x%02X Time=%llu\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_counter(header_info_t info, counter_iterator_t *it, void *arg)
{
    printf("Counters:\n");
    printf("Qualifier: ");
    print_qualifier(info.qualifier);
    printf("\n");
    printf("Variation: ");
    print_variation(info.variation);
    printf("\n");

    counter_t *value = NULL;
    while (value = counter_next(it)) {
        printf("Counter %u: Value=%u Flags=0x%02X Time=%llu\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_frozen_counter(header_info_t info, frozen_counter_iterator_t *it, void *arg)
{
    printf("Frozen Counters:\n");
    printf("Qualifier: ");
    print_qualifier(info.qualifier);
    printf("\n");
    printf("Variation: ");
    print_variation(info.variation);
    printf("\n");

    frozen_counter_t *value = NULL;
    while (value = frozencounter_next(it)) {
        printf("Frozen Counter %u: Value=%u Flags=0x%02X Time=%llu\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_analog(header_info_t info, analog_iterator_t *it, void *arg)
{
    printf("Analogs:\n");
    printf("Qualifier: ");
    print_qualifier(info.qualifier);
    printf("\n");
    printf("Variation: ");
    print_variation(info.variation);
    printf("\n");

    analog_t *value = NULL;
    while (value = analog_next(it)) {
        printf("AI %u: Value=%f Flags=0x%02X Time=%llu\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_analog_output_status(header_info_t info, analog_output_status_iterator_t *it, void *arg)
{
    printf("Analog Output Statuses:\n");
    printf("Qualifier: ");
    print_qualifier(info.qualifier);
    printf("\n");
    printf("Variation: ");
    print_variation(info.variation);
    printf("\n");

    analog_output_status_t *value = NULL;
    while (value = analogoutputstatus_next(it)) {
        printf("AOS %u: Value=%f Flags=0x%02X Time=%llu\n", value->index, value->value, value->flags.value, value->time.value);
    }
}

void handle_octet_strings(header_info_t info, octet_string_iterator_t *it, void *arg)
{
    printf("Octet Strings:\n");
    printf("Qualifier: ");
    print_qualifier(info.qualifier);
    printf("\n");
    printf("Variation: ");
    print_variation(info.variation);
    printf("\n");

    octet_string_t *value = NULL;
    while (value = octetstring_next(it)) {
        printf("Octet String: %u: Value=", value->index);
        byte_t *single_byte = byte_next(value->value);
        while (single_byte != NULL) {
            printf("%02X", single_byte->value);
            single_byte = byte_next(value->value);
        }

        printf("\n");
    }
}

// Single read callback
void on_read_complete(read_result_t result, void *arg) { printf("ReadResult: %s\n", ReadResult_to_string(result)); }

// Command callback
void on_command_complete(command_result_t result, void *arg) { printf("CommandResult: %s\n", CommandResult_to_string(result)); }

// Timesync callback
void on_timesync_complete(time_sync_result_t result, void *arg) { printf("TimeSyncResult: %s\n", TimeSyncResult_to_string(result)); }

// Restart callback
void on_restart_complete(restart_result_t result, void *arg) { printf("RestartResult: %s\n", RestartSuccess_to_string(result.success)); }

void on_link_status_complete(link_status_result_t result, void *arg) { printf("LinkStatusResult: %s\n", LinkStatusResult_to_string(result)); }

// Timestamp callback
time_provider_timestamp_t get_time(void *arg)
{
    time_t timer = time(NULL);

    return timeprovidertimestamp_valid(timer * 1000);
}

int main()
{
    // ANCHOR: logging_init
    // define logger callback "interface"
    logger_t logger = {
        // function pointer where log messages will be sent
        .on_message = &on_log_message,
        // optional context argument applied to all log callbacks
        .ctx = NULL,
    };

    // initialize logging with the default configuration
    configure_logging(logging_config_init(), logger);
    // ANCHOR_END: logging_init

    // ANCHOR: runtime_init
    // create the runtime
    runtime_config_t runtime_config = {
        .num_core_threads = 4,
    };
    runtime_t *runtime = NULL;
    if(runtime_new(runtime_config, &runtime) != Dnp3Error_Ok)
        goto cleanup;
    // ANCHOR_END: runtime_init

    // Create the master
    master_config_t master_config = master_config_init(1);
    master_config.decode_level.application = AppDecodeLevel_ObjectValues;

    endpoint_list_t *endpoints = endpoint_list_new("127.0.0.1:20000");
    client_state_listener_t listener = {
        .on_change = &client_state_on_change,
        .ctx = NULL,
    };
    master_t *master = NULL;
    if(master_create_tcp_session(runtime, LinkErrorMode_Close, master_config, endpoints, retry_strategy_init(), 1000, listener, &master) != Dnp3Error_Ok)
        goto cleanup;

    endpoint_list_destroy(endpoints);

    // Create the association
    read_handler_t read_handler = {
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
        .ctx = NULL,
    };

    association_config_t association_config = association_config_init(
        // disable unsolicited (Class 1/2/3)
        event_classes_all(),
        // after the integrity poll, enable unsolicited (Class 1/2/3)
        event_classes_all(),
        // perform an integrity poll with Class 1/2/3/0
        classes_all(),
        // don't automatically scan Class 1/2/3 when the corresponding IIN bit is asserted
        event_classes_none());
    association_config.auto_time_sync = AutoTimeSync_Lan;
    association_config.keep_alive_timeout = 60;

    time_provider_t time_provider = {
        .get_time = get_time,
        .ctx = NULL,
    };
    association_id_t association_id;
    if(master_add_association(master, 1024, association_config, read_handler, time_provider, &association_id) != Dnp3Error_Ok)
        goto cleanup;

    // Add an event poll
    request_t *poll_request = request_new_class(false, true, true, true);
    poll_id_t poll_id;
    if(master_add_poll(master, association_id, poll_request, 5000, &poll_id) != Dnp3Error_Ok)
        goto cleanup;
    request_destroy(poll_request);

    // start communications
    master_enable(master);

    char cbuf[10];
    while (true) {
        fgets(cbuf, 10, stdin);

        if (strcmp(cbuf, "x\n") == 0) {
            goto cleanup;
        }
        else if (strcmp(cbuf, "enable\n") == 0) {
            printf("calling enable\n");
            master_enable(master);
        }
        else if (strcmp(cbuf, "disable\n") == 0) {
            printf("calling disable\n");
            master_disable(master);
        }
        else if (strcmp(cbuf, "dln\n") == 0) {
            master_set_decode_level(master, decode_level_init());
        }
        else if (strcmp(cbuf, "dlv\n") == 0) {
            decode_level_t level = decode_level_init();
            level.application = AppDecodeLevel_ObjectValues;
            master_set_decode_level(master, level);
        }
        else if (strcmp(cbuf, "rao\n") == 0) {
            request_t *request = request_new();
            request_add_all_objects_header(request, Variation_Group40Var0);

            read_task_callback_t cb = {
                .on_complete = &on_read_complete,
                .ctx = NULL,
            };
            master_read(master, association_id, request, cb);

            request_destroy(request);
        }
        else if (strcmp(cbuf, "rmo\n") == 0) {
            request_t *request = request_new();
            request_add_all_objects_header(request, Variation_Group10Var0);
            request_add_all_objects_header(request, Variation_Group40Var0);

            read_task_callback_t cb = {
                .on_complete = &on_read_complete,
                .ctx = NULL,
            };
            master_read(master, association_id, request, cb);

            request_destroy(request);
        }
        else if (strcmp(cbuf, "cmd\n") == 0) {
            commands_t *commands = commands_new();
            g12v1_t g12v1 = g12v1_init(control_code_init(TripCloseCode_Nul, false, OpType_LatchOn), 1, 1000, 1000);
            commands_add_g12v1_u16(commands, 3, g12v1);

            command_task_callback_t cb = {
                .on_complete = &on_command_complete,
                .ctx = NULL,
            };

            master_operate(master, association_id, CommandMode_SelectBeforeOperate, commands, cb);

            commands_destroy(commands);
        }
        else if (strcmp(cbuf, "evt\n") == 0) {
            master_demand_poll(master, poll_id);
        }
        else if (strcmp(cbuf, "lts\n") == 0) {
            time_sync_task_callback_t cb = {
                .on_complete = &on_timesync_complete,
                .ctx = NULL,
            };
            master_sync_time(master, association_id, TimeSyncMode_Lan, cb);
        }
        else if (strcmp(cbuf, "nts\n") == 0) {
            time_sync_task_callback_t cb = {
                .on_complete = &on_timesync_complete,
                .ctx = NULL,
            };
            master_sync_time(master, association_id, TimeSyncMode_NonLan, cb);
        }
        else if (strcmp(cbuf, "crt\n") == 0) {
            restart_task_callback_t cb = {
                .on_complete = &on_restart_complete,
                .ctx = NULL,
            };
            master_cold_restart(master, association_id, cb);
        }
        else if (strcmp(cbuf, "wrt\n") == 0) {
            restart_task_callback_t cb = {
                .on_complete = &on_restart_complete,
                .ctx = NULL,
            };
            master_warm_restart(master, association_id, cb);
        }
        else if (strcmp(cbuf, "lsr\n") == 0) {
            link_status_callback_t cb = {
                .on_complete = &on_link_status_complete,
                .ctx = NULL,
            };
            master_check_link_status(master, association_id, cb);
        }
        else {
            printf("Unknown command\n");
        }
    }

    // Cleanup
cleanup:
    if(master)
        master_destroy(master);
    // ANCHOR: runtime_destroy
    if(runtime)
        runtime_destroy(runtime);
    // ANCHOR_END: runtime_destroy

    return 0;
}
