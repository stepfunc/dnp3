#include "dnp3.h"

#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <time.h>

dnp3_timestamp_t now()
{
    return dnp3_timestamp_synchronized_timestamp((uint64_t)time(NULL));
}

// Logger inteface
void on_log_message(dnp3_log_level_t level, const char *msg, void *arg) { printf("%s", msg); }

dnp3_logger_t get_logger()
{
    return (dnp3_logger_t){
        .on_message = &on_log_message,
        .on_destroy = NULL,
        .ctx = NULL,
    };
}

// OutstationApplication interface
uint16_t get_processing_delay_ms(void *context) { return 0; }

dnp3_write_time_result_t write_absolute_time(uint64_t time, void *context) { return DNP3_WRITE_TIME_RESULT_NOT_SUPPORTED; }

dnp3_application_iin_t get_application_iin(void *context) { return dnp3_application_iin_init(); }

dnp3_restart_delay_t cold_restart(void *context) { return dnp3_restart_delay_seconds(60); }

dnp3_restart_delay_t warm_restart(void *context) { return dnp3_restart_delay_not_supported(); }

dnp3_freeze_result_t freeze_counters_all(dnp3_freeze_type_t freeze_type, dnp3_database_handle_t *database, void *context) { return DNP3_FREEZE_RESULT_NOT_SUPPORTED; }

dnp3_freeze_result_t freeze_counters_range(uint16_t start, uint16_t stop, dnp3_freeze_type_t freeze_type, dnp3_database_handle_t *database, void *context)
{
    return DNP3_FREEZE_RESULT_NOT_SUPPORTED;
}

bool write_string_attr(uint8_t set, uint8_t var, dnp3_string_attr_t attr, const char* value, void *context)
{
    // Allow writing any string attributes that have been defined as writable    
    return true;
}

dnp3_outstation_application_t get_outstation_application()
{
    return (dnp3_outstation_application_t){
        .get_processing_delay_ms = &get_processing_delay_ms,
        .write_absolute_time = &write_absolute_time,
        .get_application_iin = &get_application_iin,
        .cold_restart = &cold_restart,
        .warm_restart = &warm_restart,
        .freeze_counters_all = &freeze_counters_all,
        .freeze_counters_range = &freeze_counters_range,
        .write_string_attr = &write_string_attr,
        .on_destroy = NULL,
        .ctx = NULL,
    };
}

// OutstationInformation interface
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

dnp3_outstation_information_t get_outstation_information()
{
    return (dnp3_outstation_information_t){.process_request_from_idle = &process_request_from_idle,
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
                                           .ctx = NULL};
}

typedef struct binary_output_update_t {
    uint16_t index;
    bool status;
} binary_output_update_t;

void update_binary_output(dnp3_database_t* db, void* context)
{
    binary_output_update_t* ctx = (binary_output_update_t*)context;

    dnp3_binary_output_status_t status = dnp3_binary_output_status_init(ctx->index, ctx->status, dnp3_flags_init(DNP3_FLAG_ONLINE), now());

    dnp3_database_update_binary_output_status(db, status, dnp3_update_options_detect_event());
}

typedef struct analog_output_update_t {
    uint16_t index;
    double value;
} analog_output_update_t;

void update_analog_output_status(dnp3_database_t* db, void* context)
{
    analog_output_update_t* ctx = (analog_output_update_t*)context;

    dnp3_analog_output_status_t status = dnp3_analog_output_status_init(ctx->index, ctx->value, dnp3_flags_init(DNP3_FLAG_ONLINE), now());

    dnp3_database_update_analog_output_status(db, status, dnp3_update_options_detect_event());
}



// ControlHandler interface
// ANCHOR: control_handler

void update_binary_output_status_from_control(dnp3_database_t *database, void *ctx)
{
    dnp3_binary_output_status_t value = *(dnp3_binary_output_status_t *)ctx;
    dnp3_database_update_binary_output_status(database, value, dnp3_update_options_detect_event());
}

void update_analog_output_status_from_control(dnp3_database_t *database, void *ctx)
{
    dnp3_analog_output_status_t value = *(dnp3_analog_output_status_t *)ctx;
    dnp3_database_update_analog_output_status(database, value, dnp3_update_options_detect_event());
}

void begin_fragment(void *context) {}

void end_fragment(dnp3_database_handle_t *database, void *context) {}

dnp3_command_status_t select_g12v1(dnp3_group12_var1_t control, uint16_t index, dnp3_database_handle_t *database, void *context)
{
    if (index < 10 && (control.code.op_type == DNP3_OP_TYPE_LATCH_ON || control.code.op_type == DNP3_OP_TYPE_LATCH_OFF))
    {
        return DNP3_COMMAND_STATUS_SUCCESS;
    }
    else
    {
        return DNP3_COMMAND_STATUS_NOT_SUPPORTED;
    }
}

dnp3_command_status_t operate_g12v1(dnp3_group12_var1_t control, uint16_t index, dnp3_operate_type_t op_type, dnp3_database_handle_t *database, void *context)
{
    if (index < 10 && (control.code.op_type == DNP3_OP_TYPE_LATCH_ON || control.code.op_type == DNP3_OP_TYPE_LATCH_OFF))
    {
        bool status = (control.code.op_type == DNP3_OP_TYPE_LATCH_ON);
        dnp3_binary_output_status_t bo = dnp3_binary_output_status_init(index, status, dnp3_flags_init(DNP3_FLAG_ONLINE), now());
        dnp3_database_transaction_t transaction = {
            .execute = &update_binary_output_status_from_control,
            .on_destroy = NULL,
            .ctx = &bo,
        };
        dnp3_database_handle_transaction(database, transaction);
        return DNP3_COMMAND_STATUS_SUCCESS;
    }
    else
    {
        return DNP3_COMMAND_STATUS_NOT_SUPPORTED;
    }
}

dnp3_command_status_t select_analog_output(uint16_t index)
{
    return (index < 10) ? DNP3_COMMAND_STATUS_SUCCESS : DNP3_COMMAND_STATUS_NOT_SUPPORTED;
}

dnp3_command_status_t operate_analog_output(double value, uint16_t index, dnp3_database_handle_t *database)
{
    if (index < 10)
    {
        dnp3_analog_output_status_t ao = dnp3_analog_output_status_init(index, value, dnp3_flags_init(DNP3_FLAG_ONLINE), now());
        dnp3_database_transaction_t transaction = {
            .execute = &update_analog_output_status_from_control,
            .on_destroy = NULL,
            .ctx = &ao,
        };
        dnp3_database_handle_transaction(database, transaction);
        return DNP3_COMMAND_STATUS_SUCCESS;
    }
    else
    {
        return DNP3_COMMAND_STATUS_NOT_SUPPORTED;
    }
}

dnp3_command_status_t select_g41v1(int32_t value, uint16_t index, dnp3_database_handle_t *database, void *context)
{
    return select_analog_output(index);
}

dnp3_command_status_t operate_g41v1(int32_t value, uint16_t index, dnp3_operate_type_t op_type, dnp3_database_handle_t *database, void *context)
{
    return operate_analog_output((double)value, index, database);
}

dnp3_command_status_t select_g41v2(int16_t value, uint16_t index, dnp3_database_handle_t *database, void *context) {
    return select_analog_output(index);
}

dnp3_command_status_t operate_g41v2(int16_t value, uint16_t index, dnp3_operate_type_t op_type, dnp3_database_handle_t *database, void *context)
{
    return operate_analog_output((double)value, index, database);
}

dnp3_command_status_t select_g41v3(float value, uint16_t index, dnp3_database_handle_t *database, void *context) {
    return select_analog_output(index);
}

dnp3_command_status_t operate_g41v3(float value, uint16_t index, dnp3_operate_type_t op_type, dnp3_database_handle_t *database, void *context)
{
    return operate_analog_output((double)value, index, database);
}

dnp3_command_status_t select_g41v4(double value, uint16_t index, dnp3_database_handle_t *database, void *context) {
    return select_analog_output(index);
}

dnp3_command_status_t operate_g41v4(double value, uint16_t index, dnp3_operate_type_t op_type, dnp3_database_handle_t *database, void *context)
{
    return operate_analog_output(value, index, database);
}
// ANCHOR_END: control_handler

dnp3_control_handler_t get_control_handler()
{
    return (dnp3_control_handler_t){
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
        .on_destroy = NULL,
        .ctx = NULL,
    };
}

// ANCHOR: database_init_transaction
void outstation_transaction_startup(dnp3_database_t *db, void *context)
{
    // initialize 10 values for each type
    for (uint16_t i = 0; i < 10; ++i) {
        // you can explicitly specify the configuration for each point ...
        dnp3_database_add_binary_input(db, i, DNP3_EVENT_CLASS_CLASS1,
            dnp3_binary_input_config_create(DNP3_STATIC_BINARY_INPUT_VARIATION_GROUP1_VAR1, DNP3_EVENT_BINARY_INPUT_VARIATION_GROUP2_VAR2)
        );
        // ... or just use the defaults
        dnp3_database_add_double_bit_binary_input(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_double_bit_binary_input_config_init());
        dnp3_database_add_binary_output_status(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_binary_output_status_config_init());
        dnp3_database_add_counter(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_counter_config_init());
        dnp3_database_add_frozen_counter(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_frozen_counter_config_init());
        dnp3_database_add_analog_input(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_analog_input_config_init());
        dnp3_database_add_analog_output_status(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_analog_output_status_config_init());
        dnp3_database_add_octet_string(db, i, DNP3_EVENT_CLASS_CLASS1);
    }

    // define device attributes made available to the master
    dnp3_database_define_string_attr(db, 0, false, DNP3_ATTRIBUTE_VARIATIONS_DEVICE_MANUFACTURERS_NAME, "Step Function I/O");
    dnp3_database_define_string_attr(db, 0, true, DNP3_ATTRIBUTE_VARIATIONS_USER_ASSIGNED_LOCATION, "Bend, OR");   
}
// ANCHOR_END: database_init_transaction

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

    dnp3_binary_input_t value =
        dnp3_binary_input_init(7, ((database_points_t *)context)->binaryValue, dnp3_flags_init(DNP3_FLAG_ONLINE), now());
    dnp3_database_update_binary_input(db, value, dnp3_update_options_detect_event());
}

void double_bit_binary_transaction(dnp3_database_t *db, void *context)
{
    ((database_points_t *)context)->doubleBitBinaryValue =
        ((database_points_t *)context)->doubleBitBinaryValue == DNP3_DOUBLE_BIT_DETERMINED_OFF ? DNP3_DOUBLE_BIT_DETERMINED_ON : DNP3_DOUBLE_BIT_DETERMINED_OFF;

    dnp3_double_bit_binary_input_t value = dnp3_double_bit_binary_input_init(7, ((database_points_t *)context)->doubleBitBinaryValue,
                                                                             dnp3_flags_init(DNP3_FLAG_ONLINE), now());
    dnp3_database_update_double_bit_binary_input(db, value, dnp3_update_options_detect_event());
}

void binary_output_status_transaction(dnp3_database_t *db, void *context)
{
    ((database_points_t *)context)->binaryOutputStatusValue = !((database_points_t *)context)->binaryOutputStatusValue;

    dnp3_binary_output_status_t value = dnp3_binary_output_status_init(7, ((database_points_t *)context)->binaryOutputStatusValue,
                                                                       dnp3_flags_init(DNP3_FLAG_ONLINE), now());
    dnp3_database_update_binary_output_status(db, value, dnp3_update_options_detect_event());
}

void counter_transaction(dnp3_database_t *db, void *context)
{
    dnp3_counter_t value =
        dnp3_counter_init(7, ++((database_points_t *)context)->counterValue, dnp3_flags_init(DNP3_FLAG_ONLINE), now());
    dnp3_database_update_counter(db, value, dnp3_update_options_detect_event());
}

void frozen_counter_transaction(dnp3_database_t *db, void *context)
{
    dnp3_frozen_counter_t value =
        dnp3_frozen_counter_init(7, ++((database_points_t *)context)->frozenCounterValue, dnp3_flags_init(DNP3_FLAG_ONLINE), now());
    dnp3_database_update_frozen_counter(db, value, dnp3_update_options_detect_event());
}

void analog_transaction(dnp3_database_t *db, void *context)
{
    dnp3_analog_input_t value =
        dnp3_analog_input_init(7, ++((database_points_t *)context)->analogValue, dnp3_flags_init(DNP3_FLAG_ONLINE), now());
    dnp3_database_update_analog_input(db, value, dnp3_update_options_detect_event());
}

void analog_output_status_transaction(dnp3_database_t *db, void *context)
{
    dnp3_analog_output_status_t value = dnp3_analog_output_status_init(7, ++((database_points_t *)context)->analogOutputStatusValue,
                                                                       dnp3_flags_init(DNP3_FLAG_ONLINE), now());
    dnp3_database_update_analog_output_status(db, value, dnp3_update_options_detect_event());
}

void octet_string_transaction(dnp3_database_t *db, void *context)
{
    dnp3_octet_string_value_t *octet_string = dnp3_octet_string_value_create();
    dnp3_octet_string_value_add(octet_string, 0x48); // H
    dnp3_octet_string_value_add(octet_string, 0x65); // e
    dnp3_octet_string_value_add(octet_string, 0x6C); // l
    dnp3_octet_string_value_add(octet_string, 0x6C); // l
    dnp3_octet_string_value_add(octet_string, 0x6F); // o
    dnp3_octet_string_value_add(octet_string, 0x00); // \0

    dnp3_database_update_octet_string(db, 7, octet_string, dnp3_update_options_detect_event());

    dnp3_octet_string_value_destroy(octet_string);
}

void on_connection_state_change(dnp3_connection_state_t state, void *ctx) { printf("Connection state change: %s\n", dnp3_connection_state_to_string(state)); }

void on_port_state_change(dnp3_port_state_t state, void *ctx) { printf("Port state change: %s\n", dnp3_port_state_to_string(state)); }

dnp3_connection_state_listener_t get_connection_state_listener()
{
    return (dnp3_connection_state_listener_t){
        .on_change = &on_connection_state_change,
        .on_destroy = NULL,
        .ctx = NULL,
    };
}

dnp3_port_state_listener_t get_port_state_listener()
{
    return (dnp3_port_state_listener_t){
        .on_change = &on_port_state_change,
        .on_destroy = NULL,
        .ctx = NULL,
    };
}

// loop that accepts user input and updates values
int run_outstation(dnp3_outstation_t *outstation)
{
    database_points_t database_points = {
        .binaryValue = false,
        .doubleBitBinaryValue = DNP3_DOUBLE_BIT_DETERMINED_OFF,
        .binaryOutputStatusValue = false,
        .counterValue = 0,
        .frozenCounterValue = 0,
        .analogValue = 0.0,
        .analogOutputStatusValue = 0.0,
    };

    char cbuf[10];
    while (true) {
        fgets(cbuf, 10, stdin);

        if (strcmp(cbuf, "x\n") == 0) {
            return 0;
        }
        else if (strcmp(cbuf, "enable\n") == 0) {
            dnp3_outstation_enable(outstation);
        }
        else if (strcmp(cbuf, "disable\n") == 0) {
            dnp3_outstation_disable(outstation);
        }
        else if (strcmp(cbuf, "bi\n") == 0) {
            dnp3_database_transaction_t transaction = {
                .execute = &binary_transaction,
                .on_destroy = NULL,
                .ctx = &database_points,
            };
            dnp3_outstation_transaction(outstation, transaction);
        }
        else if (strcmp(cbuf, "dbbi\n") == 0) {
            dnp3_database_transaction_t transaction = {
                .execute = &double_bit_binary_transaction,
                .on_destroy = NULL,
                .ctx = &database_points,
            };
            dnp3_outstation_transaction(outstation, transaction);
        }
        else if (strcmp(cbuf, "bos\n") == 0) {
            dnp3_database_transaction_t transaction = {
                .execute = &binary_output_status_transaction,
                .on_destroy = NULL,
                .ctx = &database_points,
            };
            dnp3_outstation_transaction(outstation, transaction);
        }
        else if (strcmp(cbuf, "co\n") == 0) {
            dnp3_database_transaction_t transaction = {
                .execute = &counter_transaction,
                .on_destroy = NULL,
                .ctx = &database_points,
            };
            dnp3_outstation_transaction(outstation, transaction);
        }
        else if (strcmp(cbuf, "fco\n") == 0) {
            dnp3_database_transaction_t transaction = {
                .execute = &frozen_counter_transaction,
                .on_destroy = NULL,
                .ctx = &database_points,
            };
            dnp3_outstation_transaction(outstation, transaction);
        }
        else if (strcmp(cbuf, "ai\n") == 0) {
            dnp3_database_transaction_t transaction = {
                .execute = &analog_transaction,
                .on_destroy = NULL,
                .ctx = &database_points,
            };
            dnp3_outstation_transaction(outstation, transaction);
        }
        else if (strcmp(cbuf, "aos\n") == 0) {
            dnp3_database_transaction_t transaction = {
                .execute = &analog_output_status_transaction,
                .on_destroy = NULL,
                .ctx = &database_points,
            };
            dnp3_outstation_transaction(outstation, transaction);
        }
        else if (strcmp(cbuf, "os\n") == 0) {
            dnp3_database_transaction_t transaction = {
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
}

// ANCHOR: event_buffer_config
dnp3_event_buffer_config_t get_event_buffer_config()
{
    return dnp3_event_buffer_config_init(10, // binary
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

// ANCHOR: create_outstation_config
dnp3_outstation_config_t get_outstation_config()
{
    // create an outstation configuration with default values
    dnp3_outstation_config_t config = dnp3_outstation_config_init(
        // outstation address
        1024,
        // master address
        1,
        // event buffer sizes
        get_event_buffer_config()
    );
    // override the default application decoding level
    config.decode_level.application = DNP3_APP_DECODE_LEVEL_OBJECT_VALUES;
    return config;
}
// ANCHOR_END: create_outstation_config

dnp3_tls_server_config_t get_tls_self_signed_config()
{
    // ANCHOR: tls_self_signed_config
    dnp3_tls_server_config_t config = dnp3_tls_server_config_init(
        "test.com",
        "./certs/self_signed/entity1_cert.pem",
        "./certs/self_signed/entity2_cert.pem",
        "./certs/self_signed/entity2_key.pem",
        "" // no password
    );
    config.certificate_mode = DNP3_CERTIFICATE_MODE_SELF_SIGNED;
    // ANCHOR_END: tls_self_signed_config
    return config;
}

dnp3_tls_server_config_t get_tls_ca_config()
{
    // ANCHOR: tls_ca_chain_config
    dnp3_tls_server_config_t config = dnp3_tls_server_config_init(
        "test.com",
        "./certs/ca_chain/ca_cert.pem",
        "./certs/ca_chain/entity2_cert.pem",
        "./certs/ca_chain/entity2_key.pem",
        "" // no password
    );
    // ANCHOR_END: tls_ca_chain_config
    return config;
}

void init_database(dnp3_outstation_t *outstation)
{
    // setup initial points
    // ANCHOR: database_init
    dnp3_database_transaction_t startup_transaction = {
        .execute = &outstation_transaction_startup,
        .on_destroy = NULL,
        .ctx = NULL,
    };
    dnp3_outstation_transaction(outstation, startup_transaction);
    // ANCHOR_END: database_init
}

int run_server(dnp3_outstation_server_t *server)
{
    dnp3_outstation_t *outstation = NULL;
    // ANCHOR: tcp_server_add_outstation
    dnp3_address_filter_t *address_filter = dnp3_address_filter_any();
    dnp3_param_error_t err = dnp3_outstation_server_add_outstation(
        server,
        get_outstation_config(),
        get_outstation_application(),
        get_outstation_information(),
        get_control_handler(),
        get_connection_state_listener(),
        address_filter,
        &outstation
    );
    dnp3_address_filter_destroy(address_filter);
    // ANCHOR_END: tcp_server_add_outstation

    if (err) {
        printf("unable to add outstation: %s \n", dnp3_param_error_to_string(err));
        return -1;
    }

    init_database(outstation);

    // Start the server
    // ANCHOR: tcp_server_bind
    err = dnp3_outstation_server_bind(server);
    // ANCHOR_END: tcp_server_bind
    if (err) {
        printf("unable to bind server: %s \n", dnp3_param_error_to_string(err));
        // cleanup the outstation before exit
        dnp3_outstation_destroy(outstation);
        return -1;
    }

    int ret = run_outstation(outstation);
    // cleanup the outstation before exit
    dnp3_outstation_destroy(outstation);
    return ret;
}

int run_tcp_server(dnp3_runtime_t *runtime)
{
    // ANCHOR: create_tcp_server
    dnp3_outstation_server_t* server = NULL;
    dnp3_param_error_t err = dnp3_outstation_server_create_tcp_server(runtime, DNP3_LINK_ERROR_MODE_CLOSE, "127.0.0.1:20000", &server);
    // ANCHOR_END: create_tcp_server

    if (err) {    
        printf("unable to create TCP server: %s \n", dnp3_param_error_to_string(err));
        return -1;
    }

    int ret = run_server(server);
    dnp3_outstation_server_destroy(server);
    return ret;
}

int run_serial(dnp3_runtime_t* runtime)
{
    // ANCHOR: create_serial_server
    dnp3_outstation_t* outstation = NULL;
    dnp3_param_error_t err = dnp3_outstation_create_serial_session_2(
        runtime,
        "/dev/pts/4",                // change to a real port
        dnp3_serial_settings_init(), // default settings
        5000,                        // retry the port every 5 seconds
        get_outstation_config(),
        get_outstation_application(),
        get_outstation_information(),
        get_control_handler(),
        get_port_state_listener(),
        &outstation
    );
    // ANCHOR_END: create_serial_server

    if (err) {
        printf("unable to create serial outstation: %s \n", dnp3_param_error_to_string(err));
        return -1;
    }

    return run_outstation(outstation);
}

int run_tls_server(dnp3_runtime_t *runtime, dnp3_tls_server_config_t config)
{
    // ANCHOR: create_tls_server
    dnp3_outstation_server_t *server = NULL;
    dnp3_param_error_t err = dnp3_outstation_server_create_tls_server(runtime, DNP3_LINK_ERROR_MODE_CLOSE, "127.0.0.1:20001", config, &server);
    // ANCHOR_END: create_tls_server

    if (err) {
        printf("unable to create TLS server: %s \n", dnp3_param_error_to_string(err));
        return -1;
    }

    int ret = run_server(server);
    dnp3_outstation_server_destroy(server);
    return ret;
}

int run_transport(int argc, char *argv[], dnp3_runtime_t* runtime)
{    
    if (argc != 2) {
        printf("you must specify a transport type\n");
        printf("usage: outstation-example <channel> (tcp, serial, tls-ca, tls-self-signed)\n");
        return -1;
    }

    const char* type = argv[1];

    if (strcmp(type, "tcp") == 0) {
        return run_tcp_server(runtime);
    }
    else if (strcmp(type, "serial") == 0) {
        return run_serial(runtime);
    }
    else if (strcmp(type, "tls-ca") == 0) {
        return run_tls_server(runtime, get_tls_ca_config());
    }
    else if (strcmp(type, "tls-self-signed") == 0) {
        return run_tls_server(runtime, get_tls_self_signed_config());
    }
    else {
        printf("unknown channel type: %s\n", argv[1]);
        return -1;
    }
}

int main(int argc, char *argv[])
{
    // initialize logging with the default configuration
    dnp3_configure_logging(dnp3_logging_config_init(), get_logger());
    
    // ANCHOR: runtime_decl
    dnp3_runtime_t *runtime = NULL;
    // ANCHOR_END: runtime_decl
   
    // Create runtime
    dnp3_runtime_config_t runtime_config = dnp3_runtime_config_init();
    runtime_config.num_core_threads = 4;
    dnp3_param_error_t err = dnp3_runtime_create(runtime_config, &runtime);
    if (err) {
        printf("unable to create runtime: %s \n", dnp3_param_error_to_string(err));
        return -1;
    }

    // use the command line arguments to run a specific transport type
    int ret = run_transport(argc, argv, runtime);

    // cleanup the runtime before exit
    dnp3_runtime_destroy(runtime);

    return ret;
}
