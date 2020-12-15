#include "dnp3rs.h"

#include <stdio.h>
#include <string.h>
#include <time.h>

void print_qualifier(qualifier_code_t qualifier)
{
    printf(Variation_to_string(qualifier));
}

void print_variation(variation_t variation)
{
    printf(Variation_to_string(variation));
}

// Logger callback
void on_log_message(log_level_t level, const char* msg, void* arg)
{
    printf("%s: %s\n", LogLevel_to_string(level), msg);
}

// ClientState listener callback
void client_state_on_change(client_state_t state, void* arg)
{
    printf("ClientState = %s\n", ClientState_to_string(state));
}

// ReadHandler callbacks
void begin_fragment(response_header_t header, void* arg)
{
    printf("Beginning fragment (broadcast: %u)\n", iin1_is_set(&header.iin.iin1, Iin1Flag_Broadcast));
}

void end_fragment(response_header_t header, void* arg)
{
    printf("End fragment\n");
}

void handle_binary(header_info_t info, binary_iterator_t* it, void* arg)
{
    printf("Binaries:\n");
    printf("Qualifier: ");
    print_qualifier(info.qualifier);
    printf("\n");
    printf("Variation: ");
    print_variation(info.variation);
    printf("\n");

    binary_t* value = binary_next(it);
    while(value != NULL)
    {
        printf("BI %u: Value=%u Flags=0x%02X Time=%llu\n", value->index, value->value, value->flags.value, value->time.value);
        value = binary_next(it);
    }
}

void handle_double_bit_binary(header_info_t info, double_bit_binary_iterator_t* it, void* arg)
{
    printf("Double Bit Binaries:\n");
    printf("Qualifier: ");
    print_qualifier(info.qualifier);
    printf("\n");
    printf("Variation: ");
    print_variation(info.variation);
    printf("\n");

    double_bit_binary_t* value = doublebitbinary_next(it);
    while(value != NULL)
    {
        printf("DBBI %u: Value=%X Flags=0x%02X Time=%llu\n", value->index, value->value, value->flags.value, value->time.value);
        value = doublebitbinary_next(it);
    }
}

void handle_binary_output_status(header_info_t info, binary_output_status_iterator_t* it, void* arg)
{
    printf("Binary Output Statuses:\n");
    printf("Qualifier: ");
    print_qualifier(info.qualifier);
    printf("\n");
    printf("Variation: ");
    print_variation(info.variation);
    printf("\n");

    binary_output_status_t* value = binaryoutputstatus_next(it);
    while(value != NULL)
    {
        printf("BOS %u: Value=%u Flags=0x%02X Time=%llu\n", value->index, value->value, value->flags.value, value->time.value);
        value = binaryoutputstatus_next(it);
    }
}

void handle_counter(header_info_t info, counter_iterator_t* it, void* arg)
{
    printf("Counters:\n");
    printf("Qualifier: ");
    print_qualifier(info.qualifier);
    printf("\n");
    printf("Variation: ");
    print_variation(info.variation);
    printf("\n");

    counter_t* value = counter_next(it);
    while(value != NULL)
    {
        printf("Counter %u: Value=%u Flags=0x%02X Time=%llu\n", value->index, value->value, value->flags.value, value->time.value);
        value = counter_next(it);
    }
}

void handle_frozen_counter(header_info_t info, frozen_counter_iterator_t* it, void* arg)
{
    printf("Frozen Counters:\n");
    printf("Qualifier: ");
    print_qualifier(info.qualifier);
    printf("\n");
    printf("Variation: ");
    print_variation(info.variation);
    printf("\n");

    frozen_counter_t* value = frozencounter_next(it);
    while(value != NULL)
    {
        printf("Frozen Counter %u: Value=%u Flags=0x%02X Time=%llu\n", value->index, value->value, value->flags.value, value->time.value);
        value = frozencounter_next(it);
    }
}

void handle_analog(header_info_t info, analog_iterator_t* it, void* arg)
{
    printf("Analogs:\n");
    printf("Qualifier: ");
    print_qualifier(info.qualifier);
    printf("\n");
    printf("Variation: ");
    print_variation(info.variation);
    printf("\n");

    analog_t* value = analog_next(it);
    while(value != NULL)
    {
        printf("AI %u: Value=%f Flags=0x%02X Time=%llu\n", value->index, value->value, value->flags.value, value->time.value);
        value = analog_next(it);
    }
}

void handle_analog_output_status(header_info_t info, analog_output_status_iterator_t* it, void* arg)
{
    printf("Analog Output Statuses:\n");
    printf("Qualifier: ");
    print_qualifier(info.qualifier);
    printf("\n");
    printf("Variation: ");
    print_variation(info.variation);
    printf("\n");

    analog_output_status_t* value = analogoutputstatus_next(it);
    while(value != NULL)
    {
        printf("AOS %u: Value=%f Flags=0x%02X Time=%llu\n", value->index, value->value, value->flags.value, value->time.value);
        value = analogoutputstatus_next(it);
    }
}

void handle_octet_strings(header_info_t info, octet_string_iterator_t* it, void* arg)
{
    printf("Octet Strings:\n");
    printf("Qualifier: ");
    print_qualifier(info.qualifier);
    printf("\n");
    printf("Variation: ");
    print_variation(info.variation);
    printf("\n");

    octet_string_t* value = octetstring_next(it);
    while(value != NULL)
    {
        printf("Octet String: %u: Value=", value->index);
        byte_t* single_byte = byte_next(value->value);
        while(single_byte != NULL)
        {
            printf("%02X", single_byte->value);
            single_byte = byte_next(value->value);
        }

        printf("\n");
        value = octetstring_next(it);
    }
}

// Single read callback
void on_read_complete(read_result_t result, void* arg)
{
    printf("ReadResult: %s\n", ReadResult_to_string(result));
}

// Command callback
void on_command_complete(command_result_t result, void* arg)
{
    printf("CommandResult: %s\n", CommandResult_to_string(result));
}

// Timesync callback
void on_timesync_complete(time_sync_result_t result, void* arg)
{
    printf("TimeSyncResult: %s\n", TimeSyncResult_to_string(result));
}

// Restart callback
void on_restart_complete(restart_result_t result, void* arg)
{
    printf("RestartResult: %s\n", RestartSuccess_to_string(result.success));
}

void on_link_status_complete(link_status_result_t result, void* arg)
{
    printf("LinkStatusResult: %s\n", LinkStatusResult_to_string(result));
}

// Timestamp callback
time_provider_timestamp_t get_time(void* arg)
{
    time_t timer = time(NULL);

    return timeprovidertimestamp_valid(timer * 1000);
}

int main()
{
    // Setup logging
    logger_t logger =
    {
        .on_message = &on_log_message,
        .ctx = NULL,
    };
    logging_set_log_level(LogLevel_Info);
    logging_set_callback(logger);

    // Create runtime
    runtime_config_t runtime_config =
    {
        .num_core_threads = 4,
    };
    runtime_t* runtime = runtime_new(&runtime_config);

    // Create the master
    retry_strategy_t retry_strategy =
    {
        .min_delay = 100,
        .max_delay = 5000,
    };
    master_configuration_t master_config =
    {
        .address = 1,
        .level = DecodeLogLevel_ObjectValues,
        .reconnection_strategy = retry_strategy,
        .response_timeout = 5000,
        .rx_buffer_size = 2048,
        .tx_buffer_size = 2048,
    };
    client_state_listener_t listener =
    {
        .on_change = &client_state_on_change,
        .ctx = NULL,
    };
    master_t* master = runtime_add_master_tcp(
        runtime,
        master_config,
        "127.0.0.1:20000",
        listener
    );

    // Create the association
    read_handler_t read_handler =
    {
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
    association_configuration_t association_config =
    {
        .disable_unsol_classes =
        {
            .class1 = true,
            .class2 = true,
            .class3 = true,
        },
        .enable_unsol_classes =
        {
            .class1 = true,
            .class2 = true,
            .class3 = true,
        },
        .startup_integrity_classes = classes_all(),
        .auto_time_sync = AutoTimeSync_Lan,
        .auto_tasks_retry_strategy = retry_strategy,
        .keep_alive_timeout = 60,
        .auto_integrity_scan_on_buffer_overflow = true,
        .event_scan_on_events_available =
        {
            .class1 = false,
            .class2 = false,
            .class3 = false,
        }
    };
    association_handlers_t association_handlers =
    {
        .integrity_handler = read_handler,
        .unsolicited_handler = read_handler,
        .default_poll_handler = read_handler,
    };
    time_provider_t time_provider =
    {
        .get_time = get_time,
        .ctx = NULL,
    };
    association_t* association = master_add_association(
        master,
        1024,
        association_config,
        association_handlers,
        time_provider
    );

    // Add an event poll
    request_t* poll_request = request_new_class(false, true, true, true);
    poll_t* poll = association_add_poll(association, poll_request, 5000);
    request_destroy(poll_request);

    char cbuf[5];
    while(true)
    {
        fgets(cbuf, 5, stdin);

        if(strcmp(cbuf, "x\n") == 0)
        {
            goto cleanup;
        }
        else if(strcmp(cbuf, "dln\n") == 0)
        {
            master_set_decode_log_level(master, DecodeLogLevel_Nothing);
        }
        else if(strcmp(cbuf, "dlv\n") == 0)
        {
            master_set_decode_log_level(master, DecodeLogLevel_ObjectValues);
        }
        else if(strcmp(cbuf, "rao\n") == 0)
        {
            request_t* request = request_new();
            request_add_all_objects_header(request, Variation_Group40Var0);

            read_task_callback_t cb =
            {
                .on_complete = &on_read_complete,
                .ctx = NULL,
            };
            association_read(association, request, cb);

            request_destroy(request);
        }
        else if(strcmp(cbuf, "rmo\n") == 0)
        {
            request_t* request = request_new();
            request_add_all_objects_header(request, Variation_Group10Var0);
            request_add_all_objects_header(request, Variation_Group40Var0);

            read_task_callback_t cb =
            {
                .on_complete = &on_read_complete,
                .ctx = NULL,
            };
            association_read(association, request, cb);

            request_destroy(request);
        }
        else if(strcmp(cbuf, "cmd\n") == 0)
        {
            command_t* command = command_new();
            g12v1_t g12v1 =
            {
                .code =
                {
                    .tcc = TripCloseCode_Nul,
                    .clear = false,
                    .queue = false,
                    .op_type = OpType_LatchOn,
                },
                .count = 1,
                .on_time = 1000,
                .off_time = 1000,
            };
            command_add_u16_g12v1(command, 3, g12v1);

            command_task_callback_t cb =
            {
                .on_complete = &on_command_complete,
                .ctx = NULL,
            };

            association_operate(
                association,
                CommandMode_SelectBeforeOperate,
                command,
                cb
            );

            command_destroy(command);
        }
        else if(strcmp(cbuf, "evt\n") == 0)
        {
            poll_demand(poll);
        }
        else if(strcmp(cbuf, "lts\n") == 0)
        {
            time_sync_task_callback_t cb =
            {
                .on_complete = &on_timesync_complete,
                .ctx = NULL,
            };
            association_perform_time_sync(association, TimeSyncMode_Lan, cb);
        }
        else if(strcmp(cbuf, "nts\n") == 0)
        {
            time_sync_task_callback_t cb =
            {
                .on_complete = &on_timesync_complete,
                .ctx = NULL,
            };
            association_perform_time_sync(association, TimeSyncMode_NonLan, cb);
        }
        else if(strcmp(cbuf, "crt\n") == 0)
        {
            restart_task_callback_t cb =
            {
                .on_complete = &on_restart_complete,
                .ctx = NULL,
            };
            association_cold_restart(association, cb);
        }
        else if(strcmp(cbuf, "wrt\n") == 0)
        {
            restart_task_callback_t cb =
            {
                .on_complete = &on_restart_complete,
                .ctx = NULL,
            };
            association_warm_restart(association, cb);
        }
        else if(strcmp(cbuf, "lsr\n") == 0)
        {
           link_status_callback_t cb =
            {
                .on_complete = &on_link_status_complete,
                .ctx = NULL,
            };
            association_check_link_status(association, cb);
        }
        else
        {
            printf("Unknown command\n");
        }
    }

    // Cleanup
cleanup:
    poll_destroy(poll);
    association_destroy(association);
    master_destroy(master);
    runtime_destroy(runtime);

    return 0;
}
