#include "dnp3rs.h"

#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <time.h>

// Logger callback
void on_log_message(dnp3_log_level_t level, const char *msg, void *arg) { printf("%s", msg); }

// Application callbacks
uint16_t get_processing_delay_ms(void *context) { return 0; }

dnp3_write_time_result_t write_absolute_time(uint64_t time, void* context) { return DNP3_WRITE_TIME_RESULT_NOT_SUPPORTED; }

dnp3_application_iin_t get_application_iin(void* context) { return dnp3_application_iin_init(); }

dnp3_restart_delay_t cold_restart(void *context) { return dnp3_restart_delay_seconds(60); }

dnp3_restart_delay_t warm_restart(void *context) { return dnp3_restart_delay_not_supported(); }

// Outstation information callbacks
void process_request_from_idle(dnp3_request_header_t header, void *context) {}

void broadcast_received(dnp3_function_code_t function_code, dnp3_broadcast_action_t action, void *context) {}

void enter_solicited_confirm_wait(uint8_t ecsn, void *context) {}

void solicited_confirm_timeout(uint8_t ecsn, void *context) {}

void solicited_confirm_received(uint8_t ecsn, void *context) {}

void solicited_confirm_wait_new_request(void *context) {}

void wrong_solicited_confirm_seq(uint8_t ecsn, uint8_t seq, void *context) {}

void unexpected_confirm(bool unsolicited, uint8_t seq, void *context) {}

void enter_unsolicited_confirm_wait(uint8_t ecsn, void *context) {}

void unsolicited_confirm_timeout(uint8_t ecsn, bool retry, void *context) {}

void unsolicited_confirmed(uint8_t ecsn, void *context) {}

void clear_restart_iin(void *context) {}

// Control handler callbacks
void begin_fragment(void *context) {}

void end_fragment(void *context) {}

dnp3_command_status_t select_g12v1(dnp3_g12v1_t control, uint16_t index, dnp3_database_t *database, void *context) { return DNP3_COMMAND_STATUS_NOT_SUPPORTED; }

dnp3_command_status_t operate_g12v1(dnp3_g12v1_t control, uint16_t index, dnp3_operate_type_t op_type, dnp3_database_t *database, void *context)
{
    return DNP3_COMMAND_STATUS_NOT_SUPPORTED;
}

dnp3_command_status_t select_g41v1(int32_t control, uint16_t index, dnp3_database_t *database, void *context) { return DNP3_COMMAND_STATUS_NOT_SUPPORTED; }

dnp3_command_status_t operate_g41v1(int32_t control, uint16_t index, dnp3_operate_type_t op_type, dnp3_database_t *database, void *context)
{
    return DNP3_COMMAND_STATUS_NOT_SUPPORTED;
}

dnp3_command_status_t select_g41v2(int16_t control, uint16_t index, dnp3_database_t *database, void *context) { return DNP3_COMMAND_STATUS_NOT_SUPPORTED; }

dnp3_command_status_t operate_g41v2(int16_t control, uint16_t index, dnp3_operate_type_t op_type, dnp3_database_t *database, void *context)
{
    return DNP3_COMMAND_STATUS_NOT_SUPPORTED;
}

dnp3_command_status_t select_g41v3(float control, uint16_t index, dnp3_database_t *database, void *context) { return DNP3_COMMAND_STATUS_NOT_SUPPORTED; }

dnp3_command_status_t operate_g41v3(float control, uint16_t index, dnp3_operate_type_t op_type, dnp3_database_t *database, void *context)
{
    return DNP3_COMMAND_STATUS_NOT_SUPPORTED;
}

dnp3_command_status_t select_g41v4(double control, uint16_t index, dnp3_database_t *database, void *context) { return DNP3_COMMAND_STATUS_NOT_SUPPORTED; }

dnp3_command_status_t operate_g41v4(double control, uint16_t index, dnp3_operate_type_t op_type, dnp3_database_t *database, void *context)
{
    return DNP3_COMMAND_STATUS_NOT_SUPPORTED;
}

dnp3_freeze_result_t freeze_counters_all(dnp3_freeze_type_t freeze_type, dnp3_database_t* database, void* context)
{
    return DNP3_FREEZE_RESULT_NOT_SUPPORTED;
}

dnp3_freeze_result_t freeze_counters_range(uint16_t start, uint16_t stop, dnp3_freeze_type_t freeze_type, dnp3_database_t* database, void* context)
{
    return DNP3_FREEZE_RESULT_NOT_SUPPORTED;
}

// Transactions
void outstation_transaction_startup(dnp3_database_t *db, void *context)
{
    for (uint16_t i = 0; i < 10; ++i) {
        // initialize each point with default configuration
        dnp3_database_add_binary(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_binary_config_init());
        dnp3_database_add_double_bit_binary(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_double_bit_binary_config_init());
        dnp3_database_add_binary_output_status(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_binary_output_status_config_init());
        dnp3_database_add_counter(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_counter_config_init());
        dnp3_database_add_frozen_counter(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_frozen_counter_config_init());
        dnp3_database_add_analog(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_analog_config_init());
        dnp3_database_add_analog_output_status(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_analog_output_status_config_init());
        dnp3_database_add_octet_string(db, i, DNP3_EVENT_CLASS_CLASS1);

        // Set initial values
        dnp3_flags_t restart = dnp3_flags_init(DNP3_FLAG_RESTART);

        dnp3_database_update_binary(db, dnp3_binary_init(i, false, restart, dnp3_timestamp_invalid()), dnp3_update_options_init());

        dnp3_database_update_double_bit_binary(db, dnp3_double_bit_binary_init(i, DNP3_DOUBLE_BIT_DETERMINED_OFF, restart, dnp3_timestamp_invalid()), dnp3_update_options_init());

        dnp3_database_update_binary_output_status(db, dnp3_binary_output_status_init(i, false, restart, dnp3_timestamp_invalid()), dnp3_update_options_init());

        dnp3_database_update_counter(db, dnp3_counter_init(i, 0, restart, dnp3_timestamp_invalid()), dnp3_update_options_init());

        dnp3_database_update_frozen_counter(db, dnp3_frozen_counter_init(i, 0, restart, dnp3_timestamp_invalid()), dnp3_update_options_init());

        dnp3_database_update_analog(db, dnp3_analog_init(i, 0.0, restart, dnp3_timestamp_invalid()), dnp3_update_options_init());

        dnp3_database_update_analog_output_status(db, dnp3_analog_output_status_init(i, 0.0, restart, dnp3_timestamp_invalid()), dnp3_update_options_init());
    }
}

typedef struct database_points_t {
    bool binaryValue;
    dnp3_double_bit_t doubleBitBinaryValue;
    bool binaryOutputStatusValue;
    uint32_t counterValue;
    uint32_t frozenCounterValue;
    double analogValue;
    double analogOutputStatusValue;
} database_points_t;

void binary_transaction(dnp3_database_t *db, void *context)
{
    ((database_points_t *)context)->binaryValue = !((database_points_t *)context)->binaryValue;

    dnp3_binary_t value = dnp3_binary_init(7, ((database_points_t *)context)->binaryValue, dnp3_flags_init(DNP3_FLAG_ONLINE), dnp3_timestamp_synchronized(0));
    dnp3_database_update_binary(db, value, dnp3_update_options_init());
}

void double_bit_binary_transaction(dnp3_database_t *db, void *context)
{
    ((database_points_t *)context)->doubleBitBinaryValue =
        ((database_points_t *)context)->doubleBitBinaryValue == DNP3_DOUBLE_BIT_DETERMINED_OFF ? DNP3_DOUBLE_BIT_DETERMINED_ON : DNP3_DOUBLE_BIT_DETERMINED_OFF;

    dnp3_double_bit_binary_t value =
        dnp3_double_bit_binary_init(7, ((database_points_t *)context)->doubleBitBinaryValue, dnp3_flags_init(DNP3_FLAG_ONLINE), dnp3_timestamp_synchronized(0));
    dnp3_database_update_double_bit_binary(db, value, dnp3_update_options_init());
}

void binary_output_status_transaction(dnp3_database_t *db, void *context)
{
    ((database_points_t *)context)->binaryOutputStatusValue = !((database_points_t *)context)->binaryOutputStatusValue;

    dnp3_binary_output_status_t value =
        dnp3_binary_output_status_init(7, ((database_points_t *)context)->binaryOutputStatusValue, dnp3_flags_init(DNP3_FLAG_ONLINE), dnp3_timestamp_synchronized(0));
    dnp3_database_update_binary_output_status(db, value, dnp3_update_options_init());
}

void counter_transaction(dnp3_database_t *db, void *context)
{
    dnp3_counter_t value = dnp3_counter_init(7, ++((database_points_t *)context)->counterValue, dnp3_flags_init(DNP3_FLAG_ONLINE), dnp3_timestamp_synchronized(0));
    dnp3_database_update_counter(db, value, dnp3_update_options_init());
}

void frozen_counter_transaction(dnp3_database_t *db, void *context)
{
    dnp3_frozen_counter_t value = dnp3_frozen_counter_init(7, ++((database_points_t *)context)->frozenCounterValue, dnp3_flags_init(DNP3_FLAG_ONLINE), dnp3_timestamp_synchronized(0));
    dnp3_database_update_frozen_counter(db, value, dnp3_update_options_init());
}

void analog_transaction(dnp3_database_t *db, void *context)
{
    dnp3_analog_t value = dnp3_analog_init(7, ++((database_points_t *)context)->analogValue, dnp3_flags_init(DNP3_FLAG_ONLINE), dnp3_timestamp_synchronized(0));
    dnp3_database_update_analog(db, value, dnp3_update_options_init());
}

void analog_output_status_transaction(dnp3_database_t *db, void *context)
{
    dnp3_analog_output_status_t value =
        dnp3_analog_output_status_init(7, ++((database_points_t *)context)->analogOutputStatusValue, dnp3_flags_init(DNP3_FLAG_ONLINE), dnp3_timestamp_synchronized(0));
    dnp3_database_update_analog_output_status(db, value, dnp3_update_options_init());
}

void octet_string_transaction(dnp3_database_t *db, void *context)
{
    dnp3_octet_string_value_t *octet_string = dnp3_octet_string_new();
    dnp3_octet_string_add(octet_string, 0x48); // H
    dnp3_octet_string_add(octet_string, 0x65); // e
    dnp3_octet_string_add(octet_string, 0x6C); // l
    dnp3_octet_string_add(octet_string, 0x6C); // l
    dnp3_octet_string_add(octet_string, 0x6F); // o
    dnp3_octet_string_add(octet_string, 0x00); // \0

    dnp3_database_update_octet_string(db, 7, octet_string, dnp3_update_options_init());

    dnp3_octet_string_destroy(octet_string);
}

// ANCHOR: event_buffer_config
event_buffer_config_t get_event_buffer_config() {
    return event_buffer_config_init(
            10, // binary
            10, // double-bit binary
            10, // binary output status
            5,  // counter
            5,  // frozen counter
            5,  // analog
            5,  // analog output status
            3   // octet string
    );
}
// ANCHOR_END: event_buffer_config

int main()
{
    // Setup logging
    dnp3_logger_t logger = {
        .on_message = &on_log_message,
        .ctx = NULL,
    };
    // initialize logging with the default configuration
    dnp3_configure_logging(dnp3_logging_config_init(), logger);

    // types that get heap allocated and must be freed in "cleanup"
    dnp3_runtime_t* runtime = NULL;
    dnp3_tcp_server_t* server = NULL;
    dnp3_outstation_t* outstation = NULL;

    // Create runtime
    dnp3_runtime_config_t runtime_config = {
        .num_core_threads = 4,
    };
    if (dnp3_runtime_new(runtime_config, &runtime)) {        
        goto cleanup;
    }

    if (dnp3_tcpserver_new(runtime, DNP3_LINK_ERROR_MODE_CLOSE, "127.0.0.1:20000", &server)) {
        goto cleanup;
    }

    // ANCHOR: outstation_config
    // create an outstation configuration with default values
    dnp3_outstation_config_t config = dnp3_outstation_config_init(
        // outstation address
        1024,
        // master address
        1);
    // override the default application decoding level
    config.decode_level.application = DNP3_APP_DECODE_LEVEL_OBJECT_VALUES;
    // ANCHOR_END: outstation_config

    dnp3_outstation_application_t application = {
        .get_processing_delay_ms = &get_processing_delay_ms,
        .write_absolute_time = &write_absolute_time,
        .get_application_iin = &get_application_iin,
        .cold_restart = &cold_restart,
        .warm_restart = &warm_restart,
        .on_destroy = NULL,
        .ctx = NULL,
    };

    dnp3_outstation_information_t information = {
        .process_request_from_idle = &process_request_from_idle,
        .broadcast_received = &broadcast_received,
        .enter_solicited_confirm_wait = &enter_solicited_confirm_wait,
        .solicited_confirm_timeout = &solicited_confirm_timeout,
        .solicited_confirm_received = &solicited_confirm_received,
        .solicited_confirm_wait_new_request = &solicited_confirm_wait_new_request,
        .wrong_solicited_confirm_seq = &wrong_solicited_confirm_seq,
        .unexpected_confirm = &unexpected_confirm,
        .enter_unsolicited_confirm_wait = &enter_unsolicited_confirm_wait,
        .unsolicited_confirm_timeout = &unsolicited_confirm_timeout,
        .unsolicited_confirmed = &unsolicited_confirmed,
        .clear_restart_iin = &clear_restart_iin,
        .on_destroy = NULL,
        .ctx = NULL,
    };

    dnp3_control_handler_t control_handler = {
        .begin_fragment = &begin_fragment,
        .end_fragment = &end_fragment,
        .select_g12v1 = &select_g12v1,
        .operate_g12v1 = &operate_g12v1,
        .select_g41v1 = &select_g41v1,
        .operate_g41v1 = &operate_g41v1,
        .select_g41v2 = &select_g41v2,
        .operate_g41v2 = &operate_g41v2,
        .select_g41v3 = &select_g41v3,
        .operate_g41v3 = &operate_g41v3,
        .select_g41v4 = &select_g41v4,
        .operate_g41v4 = &operate_g41v4,
        .freeze_counters_all = &freeze_counters_all,
        .freeze_counters_range = &freeze_counters_range,
        .on_destroy = NULL,
        .ctx = NULL,
    };
    dnp3_address_filter_t *address_filter = dnp3_address_filter_any();
    if (dnp3_tcpserver_add_outstation(
            server,
            config,
            get_event_buffer_config(),
            application,
            information,
            control_handler,
            address_filter,
            &outstation)) {
        printf("unable to create outstation\n");
        goto cleanup;
    }        
    dnp3_address_filter_destroy(address_filter);

    // Setup initial points
    dnp3_outstation_transaction_t startup_transaction = {
        .execute = &outstation_transaction_startup,
        .on_destroy = NULL,
        .ctx = NULL,
    };
    dnp3_outstation_transaction(outstation, startup_transaction);

    // Start the outstation    
    if (dnp3_tcpserver_bind(server)) {
        printf("unable to bind server\n");
        goto cleanup;
    }

    database_points_t database_points = {
        .binaryValue = false,
        .doubleBitBinaryValue = DNP3_DOUBLE_BIT_DETERMINED_OFF,
        .binaryOutputStatusValue = false,
        .counterValue = 0,
        .frozenCounterValue = 0,
        .analogValue = 0.0,
        .analogOutputStatusValue = 0.0,
    };

    char cbuf[6];
    while (true) {
        fgets(cbuf, 6, stdin);

        if (strcmp(cbuf, "x\n") == 0) {
            goto cleanup;
        }
        else if (strcmp(cbuf, "bi\n") == 0) {
            dnp3_outstation_transaction_t transaction = {
                .execute = &binary_transaction,
                .on_destroy = NULL,
                .ctx = &database_points,
            };
            dnp3_outstation_transaction(outstation, transaction);
        }
        else if (strcmp(cbuf, "dbbi\n") == 0) {
            dnp3_outstation_transaction_t transaction = {
                .execute = &double_bit_binary_transaction,
                .on_destroy = NULL,
                .ctx = &database_points,
            };
            dnp3_outstation_transaction(outstation, transaction);
        }
        else if (strcmp(cbuf, "bos\n") == 0) {
            dnp3_outstation_transaction_t transaction = {
                .execute = &binary_output_status_transaction,
                .on_destroy = NULL,
                .ctx = &database_points,
            };
            dnp3_outstation_transaction(outstation, transaction);
        }
        else if (strcmp(cbuf, "co\n") == 0) {
            dnp3_outstation_transaction_t transaction = {
                .execute = &counter_transaction,
                .on_destroy = NULL,
                .ctx = &database_points,
            };
            dnp3_outstation_transaction(outstation, transaction);
        }
        else if (strcmp(cbuf, "fco\n") == 0) {
            dnp3_outstation_transaction_t transaction = {
                .execute = &frozen_counter_transaction,
                .on_destroy = NULL,
                .ctx = &database_points,
            };
            dnp3_outstation_transaction(outstation, transaction);
        }
        else if (strcmp(cbuf, "ai\n") == 0) {
            dnp3_outstation_transaction_t transaction = {
                .execute = &analog_transaction,
                .on_destroy = NULL,
                .ctx = &database_points,
            };
            dnp3_outstation_transaction(outstation, transaction);
        }
        else if (strcmp(cbuf, "aos\n") == 0) {
            dnp3_outstation_transaction_t transaction = {
                .execute = &analog_output_status_transaction,
                .on_destroy = NULL,
                .ctx = &database_points,
            };
            dnp3_outstation_transaction(outstation, transaction);
        }
        else if (strcmp(cbuf, "os\n") == 0) {
            dnp3_outstation_transaction_t transaction = {
                .execute = &octet_string_transaction,
                .on_destroy = NULL,
                .ctx = &database_points,
            };
            dnp3_outstation_transaction(outstation, transaction);
        }
        else {
            printf("Unknown command\n");
        }
    }
    
// all of the destroy functions are NULL-safe
cleanup:    
    dnp3_outstation_destroy(outstation);
    dnp3_tcpserver_destroy(server);
    dnp3_runtime_destroy(runtime);
    return 0;
}
