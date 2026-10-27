// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

#include "dnp3.h"

#include <inttypes.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

// DNP3 addresses
#define OUTSTATION_ADDRESS 1024
#define MASTER_ADDRESS 1

// default endpoints and serial port, the optional command line argument replaces them
#define TCP_ENDPOINT "127.0.0.1:20000"
#define TLS_ENDPOINT "127.0.0.1:20001"
#define UDP_LOCAL_ENDPOINT "127.0.0.1:20000"
#define UDP_REMOTE_ENDPOINT "127.0.0.1:20001"
#define SERIAL_PORT "/dev/ttyS0"

// TLS files, relative to the repository root
#define TLS_DOMAIN "test.com"
#define CA_CHAIN_CA_CERT "./certs/ca_chain/ca_cert.pem"
#define CA_CHAIN_ENTITY_CERT "./certs/ca_chain/entity2_cert.pem"
#define CA_CHAIN_ENTITY_KEY "./certs/ca_chain/entity2_key.pem"
#define SELF_SIGNED_PEER_CERT "./certs/self_signed/entity1_cert.pem"
#define SELF_SIGNED_ENTITY_CERT "./certs/self_signed/entity2_cert.pem"
#define SELF_SIGNED_ENTITY_KEY "./certs/self_signed/entity2_key.pem"

// number of points of each type in the example's database
#define POINT_COUNT 10

// values the console commands toggle or increment
typedef struct console_state_t {
    bool binary_input;
    dnp3_double_bit_t double_bit_binary_input;
    bool binary_output_status;
    double analog_input;
    double analog_output_status;
} console_state_t;

// state of the OutstationApplication
typedef struct outstation_application_t {
    bool need_time;
} outstation_application_t;

int run_tcp_server(dnp3_runtime_t *runtime, const char *endpoint);
int run_tcp_client(dnp3_runtime_t *runtime, const char *endpoint);
int run_udp(dnp3_runtime_t *runtime, const char *remote_endpoint);
int run_serial(dnp3_runtime_t *runtime, const char *port);
int run_tls_server(dnp3_runtime_t *runtime, const char *endpoint, dnp3_tls_server_config_t config);
int run_server(dnp3_outstation_server_t *server);
void init_database(dnp3_outstation_t *outstation);
void outstation_transaction_startup(dnp3_database_t *db, void *context);
int run_outstation(dnp3_outstation_t *outstation);
void print_help(void);
dnp3_outstation_config_t get_outstation_config(void);
dnp3_event_buffer_config_t get_event_buffer_config(void);
dnp3_tls_server_config_t get_tls_ca_config(void);
dnp3_tls_server_config_t get_tls_self_signed_config(void);
dnp3_logger_t get_logger(void);
dnp3_outstation_application_t get_outstation_application(void);
dnp3_control_handler_t get_control_handler(void);
dnp3_outstation_information_t get_outstation_information(void);
dnp3_connection_state_listener_t get_connection_state_listener(void);
dnp3_client_state_listener_t get_client_state_listener(void);
dnp3_port_state_listener_t get_port_state_listener(void);
void binary_input_transaction(dnp3_database_t *db, void *context);
void double_bit_binary_input_transaction(dnp3_database_t *db, void *context);
void binary_output_status_transaction(dnp3_database_t *db, void *context);
void counter_transaction(dnp3_database_t *db, void *context);
void frozen_counter_transaction(dnp3_database_t *db, void *context);
void analog_input_transaction(dnp3_database_t *db, void *context);
void analog_input_flags_transaction(dnp3_database_t *db, void *context);
void analog_output_status_transaction(dnp3_database_t *db, void *context);
void octet_string_transaction(dnp3_database_t *db, void *context);
void run_transaction(dnp3_outstation_t *outstation, void (*execute)(dnp3_database_t *, void *), void *context);
dnp3_timestamp_t now(void);
bool read_line(char *line, size_t size);
void print_usage(void);

int main(int argc, char *argv[])
{
    if (argc < 2 || argc > 3) {
        print_usage();
        return EXIT_FAILURE;
    }
    const char *transport = argv[1];
    // replaces the endpoint, the UDP remote endpoint or the serial port
    const char *argument = (argc == 3) ? argv[2] : NULL;

    // initialize logging with the default configuration
    dnp3_configure_logging(dnp3_logging_config_init(), get_logger());

    dnp3_runtime_t *runtime = NULL;
    dnp3_runtime_config_t runtime_config = dnp3_runtime_config_init();
    runtime_config.num_core_threads = 4;
    dnp3_param_error_t err = dnp3_runtime_create(runtime_config, &runtime);
    if (err) {
        printf("unable to create runtime: %s\n", dnp3_param_error_to_string(err));
        return EXIT_FAILURE;
    }

    int ret = EXIT_FAILURE;
    if (strcmp(transport, "tcp-server") == 0) {
        ret = run_tcp_server(runtime, argument ? argument : TCP_ENDPOINT);
    }
    else if (strcmp(transport, "tcp-client") == 0) {
        ret = run_tcp_client(runtime, argument ? argument : TCP_ENDPOINT);
    }
    else if (strcmp(transport, "udp") == 0) {
        ret = run_udp(runtime, argument ? argument : UDP_REMOTE_ENDPOINT);
    }
    else if (strcmp(transport, "serial") == 0) {
        ret = run_serial(runtime, argument ? argument : SERIAL_PORT);
    }
    else if (strcmp(transport, "tls-ca") == 0) {
        ret = run_tls_server(runtime, argument ? argument : TLS_ENDPOINT, get_tls_ca_config());
    }
    else if (strcmp(transport, "tls-self-signed") == 0) {
        ret = run_tls_server(runtime, argument ? argument : TLS_ENDPOINT, get_tls_self_signed_config());
    }
    else {
        print_usage();
    }

    // shut down everything still running on the runtime
    dnp3_runtime_destroy(runtime);
    return ret;
}

int run_tcp_server(dnp3_runtime_t *runtime, const char *endpoint)
{
    printf("Starting TCP server on %s\n", endpoint);
    // ANCHOR: create_tcp_server
    dnp3_outstation_server_t *server = NULL;
    dnp3_param_error_t err = dnp3_outstation_server_create_tcp_server(runtime, DNP3_LINK_ERROR_MODE_CLOSE, endpoint, &server);
    // ANCHOR_END: create_tcp_server

    if (err) {
        printf("unable to create TCP server: %s\n", dnp3_param_error_to_string(err));
        return EXIT_FAILURE;
    }

    int ret = run_server(server);
    dnp3_outstation_server_destroy(server);
    return ret;
}

int run_tcp_client(dnp3_runtime_t *runtime, const char *endpoint)
{
    printf("Starting TCP client to %s\n", endpoint);
    dnp3_outstation_t *outstation = NULL;
    dnp3_endpoint_list_t *endpoints = dnp3_endpoint_list_create(endpoint);
    dnp3_connect_options_t *options = dnp3_connect_options_create();
    dnp3_param_error_t err = dnp3_outstation_create_tcp_client(
        runtime,
        DNP3_LINK_ERROR_MODE_CLOSE,
        endpoints,
        dnp3_connect_strategy_init(),
        options,
        get_outstation_config(),
        get_outstation_application(),
        get_outstation_information(),
        get_control_handler(),
        get_client_state_listener(),
        &outstation
    );
    dnp3_connect_options_destroy(options);
    dnp3_endpoint_list_destroy(endpoints);

    if (err) {
        printf("unable to create TCP client: %s\n", dnp3_param_error_to_string(err));
        return EXIT_FAILURE;
    }

    init_database(outstation);
    // a TCP client outstation starts disabled: enable it to connect
    dnp3_outstation_enable(outstation);
    int ret = run_outstation(outstation);
    dnp3_outstation_destroy(outstation);
    return ret;
}

int run_udp(dnp3_runtime_t *runtime, const char *remote_endpoint)
{
    printf("Starting UDP transport from %s to %s\n", UDP_LOCAL_ENDPOINT, remote_endpoint);
    // ANCHOR: create_udp
    dnp3_outstation_t *outstation = NULL;
    dnp3_param_error_t err = dnp3_outstation_create_udp(
        runtime,
        dnp3_outstation_udp_config_init(UDP_LOCAL_ENDPOINT, remote_endpoint),
        get_outstation_config(),
        get_outstation_application(),
        get_outstation_information(),
        get_control_handler(),
        &outstation
    );
    // ANCHOR_END: create_udp

    if (err) {
        printf("unable to create UDP outstation: %s\n", dnp3_param_error_to_string(err));
        return EXIT_FAILURE;
    }

    init_database(outstation);
    int ret = run_outstation(outstation);
    dnp3_outstation_destroy(outstation);
    return ret;
}

int run_serial(dnp3_runtime_t *runtime, const char *port)
{
    printf("Starting serial transport on %s\n", port);
    // ANCHOR: create_serial_server
    dnp3_outstation_t *outstation = NULL;
    dnp3_param_error_t err = dnp3_outstation_create_serial_session_2(
        runtime,
        port,
        dnp3_serial_settings_init(), // 9600 8N1, no flow control
        1000,                        // retry opening the port every second
        get_outstation_config(),
        get_outstation_application(),
        get_outstation_information(),
        get_control_handler(),
        get_port_state_listener(),
        &outstation
    );
    // ANCHOR_END: create_serial_server

    if (err) {
        printf("unable to create serial outstation: %s\n", dnp3_param_error_to_string(err));
        return EXIT_FAILURE;
    }

    init_database(outstation);
    int ret = run_outstation(outstation);
    dnp3_outstation_destroy(outstation);
    return ret;
}

int run_tls_server(dnp3_runtime_t *runtime, const char *endpoint, dnp3_tls_server_config_t config)
{
    printf("Starting TLS server on %s\n", endpoint);
    // ANCHOR: create_tls_server
    dnp3_outstation_server_t *server = NULL;
    dnp3_param_error_t err = dnp3_outstation_server_create_tls_server(runtime, DNP3_LINK_ERROR_MODE_CLOSE, endpoint, config, &server);
    // ANCHOR_END: create_tls_server

    if (err) {
        printf("unable to create TLS server: %s\n", dnp3_param_error_to_string(err));
        return EXIT_FAILURE;
    }

    int ret = run_server(server);
    dnp3_outstation_server_destroy(server);
    return ret;
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
        printf("unable to add outstation: %s\n", dnp3_param_error_to_string(err));
        return EXIT_FAILURE;
    }

    // set up the outstation's database before the server starts accepting connections
    init_database(outstation);

    // ANCHOR: tcp_server_bind
    err = dnp3_outstation_server_bind(server);
    // ANCHOR_END: tcp_server_bind

    int ret = EXIT_FAILURE;
    if (err) {
        printf("unable to bind server: %s\n", dnp3_param_error_to_string(err));
    }
    else {
        ret = run_outstation(outstation);
    }
    dnp3_outstation_destroy(outstation);
    return ret;
}

// add the example's points and attributes to the database
void init_database(dnp3_outstation_t *outstation)
{
    // ANCHOR: database_init
    dnp3_database_transaction_t startup_transaction = {
        .execute = &outstation_transaction_startup,
        .on_destroy = NULL,
        .ctx = NULL,
    };
    dnp3_outstation_transaction(outstation, startup_transaction);
    // ANCHOR_END: database_init
}

// ANCHOR: database_init_transaction
void outstation_transaction_startup(dnp3_database_t *db, void *context)
{
    // initialize POINT_COUNT points of each type
    for (uint16_t i = 0; i < POINT_COUNT; ++i) {
        // you can explicitly specify the configuration for each point ...
        dnp3_database_add_binary_input(db, i, DNP3_EVENT_CLASS_CLASS1,
            dnp3_binary_input_config_create(DNP3_STATIC_BINARY_INPUT_VARIATION_GROUP1_VAR1, DNP3_EVENT_BINARY_INPUT_VARIATION_GROUP2_VAR2)
        );
        // ... or just use the defaults
        dnp3_database_add_double_bit_binary_input(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_double_bit_binary_input_config_init());
        dnp3_database_add_binary_output_status(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_binary_output_status_config_init());
        dnp3_database_add_counter(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_counter_config_init());
        dnp3_database_add_frozen_counter(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_frozen_counter_config_init());
        dnp3_database_add_analog_input(db, i, DNP3_EVENT_CLASS_CLASS1,
            dnp3_analog_input_config_create(DNP3_STATIC_ANALOG_INPUT_VARIATION_GROUP30_VAR1, DNP3_EVENT_ANALOG_INPUT_VARIATION_GROUP32_VAR1, 0.0)
        );
        dnp3_database_add_analog_output_status(db, i, DNP3_EVENT_CLASS_CLASS1, dnp3_analog_output_status_config_init());
        dnp3_database_add_octet_string(db, i, DNP3_EVENT_CLASS_CLASS1);
    }

    // define device attributes made available to the master
    dnp3_database_define_string_attr(db, 0, false, DNP3_ATTRIBUTE_VARIATIONS_DEVICE_MANUFACTURERS_NAME, "Step Function I/O");
    dnp3_database_define_string_attr(db, 0, true, DNP3_ATTRIBUTE_VARIATIONS_USER_ASSIGNED_LOCATION, "Bend, OR");
}
// ANCHOR_END: database_init_transaction

// run the same console loop regardless of the transport type
int run_outstation(dnp3_outstation_t *outstation)
{
    console_state_t state = {
        .binary_input = false,
        .double_bit_binary_input = DNP3_DOUBLE_BIT_DETERMINED_OFF,
        .binary_output_status = false,
        .analog_input = 0.0,
        .analog_output_status = 0.0,
    };

    print_help();

    // exit on "x" or at the end of input
    char line[64];
    while (read_line(line, sizeof(line))) {
        if (strcmp(line, "x") == 0) {
            return EXIT_SUCCESS;
        }
        else if (strcmp(line, "help") == 0) {
            print_help();
        }
        else if (strcmp(line, "enable") == 0) {
            dnp3_outstation_enable(outstation);
        }
        else if (strcmp(line, "disable") == 0) {
            dnp3_outstation_disable(outstation);
        }
        else if (strcmp(line, "bi") == 0) {
            run_transaction(outstation, &binary_input_transaction, &state);
        }
        else if (strcmp(line, "dbbi") == 0) {
            run_transaction(outstation, &double_bit_binary_input_transaction, &state);
        }
        else if (strcmp(line, "bos") == 0) {
            run_transaction(outstation, &binary_output_status_transaction, &state);
        }
        else if (strcmp(line, "co") == 0) {
            run_transaction(outstation, &counter_transaction, NULL);
        }
        else if (strcmp(line, "fco") == 0) {
            run_transaction(outstation, &frozen_counter_transaction, NULL);
        }
        else if (strcmp(line, "ai") == 0) {
            run_transaction(outstation, &analog_input_transaction, &state);
        }
        else if (strcmp(line, "aif") == 0) {
            run_transaction(outstation, &analog_input_flags_transaction, NULL);
        }
        else if (strcmp(line, "aos") == 0) {
            run_transaction(outstation, &analog_output_status_transaction, &state);
        }
        else if (strcmp(line, "os") == 0) {
            run_transaction(outstation, &octet_string_transaction, NULL);
        }
        else {
            printf("unknown command: %s\n", line);
        }
    }
    return EXIT_SUCCESS;
}

void print_help(void)
{
    printf("Commands (each updates the point at index 7):\n");
    printf("  help - print this list\n");
    printf("  x - exit\n");
    printf("  enable - enable the outstation\n");
    printf("  disable - disable the outstation\n");
    printf("  bi - toggle binary input\n");
    printf("  dbbi - toggle double bit binary input\n");
    printf("  bos - toggle binary output status\n");
    printf("  co - increment counter\n");
    printf("  fco - increment frozen counter\n");
    printf("  ai - increment analog input\n");
    printf("  aif - set analog input flag to COMM_LOST\n");
    printf("  aos - increment analog output status\n");
    printf("  os - set octet string to 'Hello'\n");
}

dnp3_outstation_config_t get_outstation_config(void)
{
    // ANCHOR: outstation_config
    dnp3_outstation_config_t config = dnp3_outstation_config_init(
        // outstation address
        OUTSTATION_ADDRESS,
        // master address
        MASTER_ADDRESS,
        // event buffer sizes
        get_event_buffer_config()
    );
    // override the default application decoding level
    config.decode_level.application = DNP3_APP_DECODE_LEVEL_OBJECT_VALUES;
    // ANCHOR_END: outstation_config
    return config;
}

// ANCHOR: event_buffer_config
dnp3_event_buffer_config_t get_event_buffer_config(void)
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

dnp3_tls_server_config_t get_tls_ca_config(void)
{
    // ANCHOR: tls_ca_chain_config
    dnp3_tls_server_config_t config = dnp3_tls_server_config_init(
        TLS_DOMAIN,
        CA_CHAIN_CA_CERT,
        CA_CHAIN_ENTITY_CERT,
        CA_CHAIN_ENTITY_KEY,
        "" // no password
    );
    // ANCHOR_END: tls_ca_chain_config
    return config;
}

dnp3_tls_server_config_t get_tls_self_signed_config(void)
{
    // ANCHOR: tls_self_signed_config
    dnp3_tls_server_config_t config = dnp3_tls_server_config_init(
        TLS_DOMAIN,
        SELF_SIGNED_PEER_CERT,
        SELF_SIGNED_ENTITY_CERT,
        SELF_SIGNED_ENTITY_KEY,
        "" // no password
    );
    config.certificate_mode = DNP3_CERTIFICATE_MODE_SELF_SIGNED;
    // ANCHOR_END: tls_self_signed_config
    return config;
}

// Logger interface
void on_log_message(dnp3_log_level_t level, const char *msg, void *context) { printf("%s", msg); }

dnp3_logger_t get_logger(void)
{
    return (dnp3_logger_t){
        .on_message = &on_log_message,
        .on_destroy = NULL,
        .ctx = NULL,
    };
}

// ANCHOR: time_sync
dnp3_application_iin_t get_application_iin(void *context)
{
    dnp3_application_iin_t iin = dnp3_application_iin_init();
    iin.need_time = ((outstation_application_t *)context)->need_time;
    return iin;
}

dnp3_write_time_result_t write_absolute_time(uint64_t time, void *context)
{
    // a real device sets its clock here, and sets need_time again when its clock drifts
    printf("write absolute time: %" PRIu64 "\n", time);
    ((outstation_application_t *)context)->need_time = false;
    return DNP3_WRITE_TIME_RESULT_OK;
}
// ANCHOR_END: time_sync

// ANCHOR: freeze
typedef struct freeze_request_t {
    uint16_t start;
    uint16_t stop;
    bool clear;
} freeze_request_t;

void freeze_transaction(dnp3_database_t *db, void *context)
{
    freeze_request_t *request = (freeze_request_t *)context;
    for (uint32_t index = request->start; index <= request->stop; ++index) {
        dnp3_counter_t counter;
        if (dnp3_database_get_counter(db, (uint16_t)index, &counter) == DNP3_PARAM_ERROR_OK) {
            dnp3_timestamp_t time = now();
            dnp3_frozen_counter_t frozen = dnp3_frozen_counter_init(counter.index, counter.value, counter.flags, time);
            dnp3_database_update_frozen_counter(db, frozen, dnp3_update_options_detect_event());
            if (request->clear) {
                dnp3_counter_t cleared = dnp3_counter_init(counter.index, 0, counter.flags, time);
                dnp3_database_update_counter(db, cleared, dnp3_update_options_detect_event());
            }
        }
    }
}

// freezing at a time and interval isn't supported: those callbacks keep their default
dnp3_freeze_result_t freeze_counters(uint16_t start, uint16_t stop, dnp3_freeze_type_t freeze_type, dnp3_database_handle_t *database)
{
    freeze_request_t request = {
        .start = start,
        .stop = (stop < POINT_COUNT - 1) ? stop : POINT_COUNT - 1,
        .clear = freeze_type == DNP3_FREEZE_TYPE_FREEZE_AND_CLEAR,
    };
    printf("freeze counters %u to %u (clear: %s)\n", request.start, request.stop, request.clear ? "true" : "false");
    dnp3_database_transaction_t transaction = {
        .execute = &freeze_transaction,
        .on_destroy = NULL,
        .ctx = &request,
    };
    dnp3_database_handle_transaction(database, transaction);
    return DNP3_FREEZE_RESULT_OK;
}

dnp3_freeze_result_t freeze_counters_all(dnp3_freeze_type_t freeze_type, dnp3_database_handle_t *database, void *context)
{
    return freeze_counters(0, POINT_COUNT - 1, freeze_type, database);
}

dnp3_freeze_result_t freeze_counters_range(uint16_t start, uint16_t stop, dnp3_freeze_type_t freeze_type, dnp3_database_handle_t *database, void *context)
{
    return freeze_counters(start, stop, freeze_type, database);
}
// ANCHOR_END: freeze

// ANCHOR: save_changes
bool support_write_analog_dead_bands(void *context) { return true; }

void begin_write_analog_dead_bands(void *context) { printf("begin writing analog dead-bands\n"); }

void write_analog_dead_band(uint16_t index, double dead_band, void *context)
{
    printf("change analog dead-band %u to %.15g\n", index, dead_band);
}

void end_write_analog_dead_bands(void *context) { printf("end writing analog dead-bands: save them here\n"); }

void begin_assign_class(void *context) { printf("begin assigning classes\n"); }

void assign_class_all(dnp3_point_type_t point_type, dnp3_event_class_t point_class, void *context)
{
    printf("assign class %s to all %s points\n", dnp3_event_class_to_string(point_class), dnp3_point_type_to_string(point_type));
}

void assign_class_range(dnp3_point_type_t point_type, uint16_t start, uint16_t stop, dnp3_event_class_t point_class, void *context)
{
    printf("assign class %s to %s points %u to %u\n", dnp3_event_class_to_string(point_class), dnp3_point_type_to_string(point_type), start,
           stop);
}

void end_assign_class(void *context) { printf("end assigning classes: save them here\n"); }
// ANCHOR_END: save_changes

bool write_string_attr(uint8_t set, uint8_t variation, dnp3_string_attr_t attr, const char *value, void *context)
{
    printf("write device attribute: %s set=%u variation=%u value=%s\n", dnp3_string_attr_to_string(attr), set, variation, value);
    // allow writing any attribute that has been defined as writable
    return true;
}

// OutstationApplication interface: callbacks left NULL keep their default behavior
dnp3_outstation_application_t get_outstation_application(void)
{
    outstation_application_t *application = malloc(sizeof(outstation_application_t));
    if (application == NULL) {
        printf("unable to allocate the outstation application\n");
        exit(EXIT_FAILURE);
    }
    // ask the master for the time as soon as it connects
    application->need_time = true;

    return (dnp3_outstation_application_t){
        .get_application_iin = &get_application_iin,
        .write_absolute_time = &write_absolute_time,
        .freeze_counters_all = &freeze_counters_all,
        .freeze_counters_range = &freeze_counters_range,
        .support_write_analog_dead_bands = &support_write_analog_dead_bands,
        .begin_write_analog_dead_bands = &begin_write_analog_dead_bands,
        .write_analog_dead_band = &write_analog_dead_band,
        .end_write_analog_dead_bands = &end_write_analog_dead_bands,
        .begin_assign_class = &begin_assign_class,
        .assign_class_all = &assign_class_all,
        .assign_class_range = &assign_class_range,
        .end_assign_class = &end_assign_class,
        .write_string_attr = &write_string_attr,
        .on_destroy = &free,
        .ctx = application,
    };
}

// ControlHandler interface
// ANCHOR: control_handler
void update_binary_output_status_from_control(dnp3_database_t *database, void *context)
{
    dnp3_binary_output_status_t value = *(dnp3_binary_output_status_t *)context;
    dnp3_database_update_binary_output_status(database, value, dnp3_update_options_detect_event());
}

void update_analog_output_status_from_control(dnp3_database_t *database, void *context)
{
    dnp3_analog_output_status_t value = *(dnp3_analog_output_status_t *)context;
    dnp3_database_update_analog_output_status(database, value, dnp3_update_options_detect_event());
}
dnp3_command_status_t select_analog_output(uint16_t index)
{
    return (index < POINT_COUNT) ? DNP3_COMMAND_STATUS_SUCCESS : DNP3_COMMAND_STATUS_NOT_SUPPORTED;
}

dnp3_command_status_t operate_analog_output(double value, uint16_t index, dnp3_database_handle_t *database)
{
    if (index < POINT_COUNT) {
        dnp3_analog_output_status_t ao = dnp3_analog_output_status_init(index, value, dnp3_flags_init(DNP3_FLAG_ONLINE), now());
        dnp3_database_transaction_t transaction = {
            .execute = &update_analog_output_status_from_control,
            .on_destroy = NULL,
            .ctx = &ao,
        };
        dnp3_database_handle_transaction(database, transaction);
        return DNP3_COMMAND_STATUS_SUCCESS;
    }
    else {
        return DNP3_COMMAND_STATUS_NOT_SUPPORTED;
    }
}

dnp3_command_status_t select_g12v1(dnp3_group12_var1_t control, uint16_t index, dnp3_database_handle_t *database, void *context)
{
    if (index < POINT_COUNT && (control.code.op_type == DNP3_OP_TYPE_LATCH_ON || control.code.op_type == DNP3_OP_TYPE_LATCH_OFF)) {
        return DNP3_COMMAND_STATUS_SUCCESS;
    }
    else {
        return DNP3_COMMAND_STATUS_NOT_SUPPORTED;
    }
}

dnp3_command_status_t operate_g12v1(dnp3_group12_var1_t control, uint16_t index, dnp3_operate_type_t op_type, dnp3_database_handle_t *database, void *context)
{
    printf("operate %s on binary output %u via %s\n", dnp3_op_type_to_string(control.code.op_type), index, dnp3_operate_type_to_string(op_type));
    if (index < POINT_COUNT && (control.code.op_type == DNP3_OP_TYPE_LATCH_ON || control.code.op_type == DNP3_OP_TYPE_LATCH_OFF)) {
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
    else {
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

dnp3_command_status_t select_g41v2(int16_t value, uint16_t index, dnp3_database_handle_t *database, void *context)
{
    return select_analog_output(index);
}

dnp3_command_status_t operate_g41v2(int16_t value, uint16_t index, dnp3_operate_type_t op_type, dnp3_database_handle_t *database, void *context)
{
    return operate_analog_output((double)value, index, database);
}

dnp3_command_status_t select_g41v3(float value, uint16_t index, dnp3_database_handle_t *database, void *context)
{
    return select_analog_output(index);
}

dnp3_command_status_t operate_g41v3(float value, uint16_t index, dnp3_operate_type_t op_type, dnp3_database_handle_t *database, void *context)
{
    return operate_analog_output((double)value, index, database);
}

dnp3_command_status_t select_g41v4(double value, uint16_t index, dnp3_database_handle_t *database, void *context)
{
    return select_analog_output(index);
}

dnp3_command_status_t operate_g41v4(double value, uint16_t index, dnp3_operate_type_t op_type, dnp3_database_handle_t *database, void *context)
{
    return operate_analog_output(value, index, database);
}

dnp3_control_handler_t get_control_handler(void)
{
    return (dnp3_control_handler_t){
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
// ANCHOR_END: control_handler

// OutstationInformation interface: every callback keeps its default (does nothing)
dnp3_outstation_information_t get_outstation_information(void)
{
    return (dnp3_outstation_information_t){
        .on_destroy = NULL,
        .ctx = NULL,
    };
}

// listeners that print each state change of a connection, client or serial port
void on_connection_state_change(dnp3_connection_state_t state, void *context)
{
    printf("connection state: %s\n", dnp3_connection_state_to_string(state));
}

void on_client_state_change(dnp3_client_state_t state, void *context) { printf("client state: %s\n", dnp3_client_state_to_string(state)); }

void on_port_state_change(dnp3_port_state_t state, void *context) { printf("port state: %s\n", dnp3_port_state_to_string(state)); }

dnp3_connection_state_listener_t get_connection_state_listener(void)
{
    return (dnp3_connection_state_listener_t){
        .on_change = &on_connection_state_change,
        .on_destroy = NULL,
        .ctx = NULL,
    };
}

dnp3_client_state_listener_t get_client_state_listener(void)
{
    return (dnp3_client_state_listener_t){
        .on_change = &on_client_state_change,
        .on_destroy = NULL,
        .ctx = NULL,
    };
}

dnp3_port_state_listener_t get_port_state_listener(void)
{
    return (dnp3_port_state_listener_t){
        .on_change = &on_port_state_change,
        .on_destroy = NULL,
        .ctx = NULL,
    };
}

// transactions run by the console commands
void binary_input_transaction(dnp3_database_t *db, void *context)
{
    console_state_t *state = (console_state_t *)context;
    state->binary_input = !state->binary_input;
    dnp3_binary_input_t value = dnp3_binary_input_init(7, state->binary_input, dnp3_flags_init(DNP3_FLAG_ONLINE), now());
    dnp3_database_update_binary_input(db, value, dnp3_update_options_detect_event());
}

void double_bit_binary_input_transaction(dnp3_database_t *db, void *context)
{
    console_state_t *state = (console_state_t *)context;
    state->double_bit_binary_input =
        (state->double_bit_binary_input == DNP3_DOUBLE_BIT_DETERMINED_OFF) ? DNP3_DOUBLE_BIT_DETERMINED_ON : DNP3_DOUBLE_BIT_DETERMINED_OFF;
    dnp3_double_bit_binary_input_t value =
        dnp3_double_bit_binary_input_init(7, state->double_bit_binary_input, dnp3_flags_init(DNP3_FLAG_ONLINE), now());
    dnp3_database_update_double_bit_binary_input(db, value, dnp3_update_options_detect_event());
}

void binary_output_status_transaction(dnp3_database_t *db, void *context)
{
    console_state_t *state = (console_state_t *)context;
    state->binary_output_status = !state->binary_output_status;
    dnp3_binary_output_status_t value = dnp3_binary_output_status_init(7, state->binary_output_status, dnp3_flags_init(DNP3_FLAG_ONLINE), now());
    dnp3_database_update_binary_output_status(db, value, dnp3_update_options_detect_event());
}

void counter_transaction(dnp3_database_t *db, void *context)
{
    // read the counter, which a freeze may have cleared
    dnp3_counter_t current = dnp3_counter_init(7, 0, dnp3_flags_init(DNP3_FLAG_ONLINE), now());
    dnp3_database_get_counter(db, 7, &current);
    dnp3_counter_t value = dnp3_counter_init(7, current.value + 1, dnp3_flags_init(DNP3_FLAG_ONLINE), now());
    dnp3_database_update_counter(db, value, dnp3_update_options_detect_event());
}

void frozen_counter_transaction(dnp3_database_t *db, void *context)
{
    // read the frozen counter, which a freeze may have changed
    dnp3_frozen_counter_t current = dnp3_frozen_counter_init(7, 0, dnp3_flags_init(DNP3_FLAG_ONLINE), now());
    dnp3_database_get_frozen_counter(db, 7, &current);
    dnp3_frozen_counter_t value = dnp3_frozen_counter_init(7, current.value + 1, dnp3_flags_init(DNP3_FLAG_ONLINE), now());
    dnp3_database_update_frozen_counter(db, value, dnp3_update_options_detect_event());
}

void analog_input_transaction(dnp3_database_t *db, void *context)
{
    console_state_t *state = (console_state_t *)context;
    state->analog_input += 1.0;
    dnp3_analog_input_t value = dnp3_analog_input_init(7, state->analog_input, dnp3_flags_init(DNP3_FLAG_ONLINE), now());
    dnp3_database_update_analog_input(db, value, dnp3_update_options_detect_event());
}

void analog_input_flags_transaction(dnp3_database_t *db, void *context)
{
    dnp3_database_update_flags(db, 7, DNP3_UPDATE_FLAGS_TYPE_ANALOG_INPUT, dnp3_flags_init(DNP3_FLAG_COMM_LOST), now(),
                               dnp3_update_options_detect_event());
}

void analog_output_status_transaction(dnp3_database_t *db, void *context)
{
    console_state_t *state = (console_state_t *)context;
    state->analog_output_status += 1.0;
    dnp3_analog_output_status_t value = dnp3_analog_output_status_init(7, state->analog_output_status, dnp3_flags_init(DNP3_FLAG_ONLINE), now());
    dnp3_database_update_analog_output_status(db, value, dnp3_update_options_detect_event());
}

void octet_string_transaction(dnp3_database_t *db, void *context)
{
    const char *text = "Hello";
    dnp3_octet_string_value_t *octet_string = dnp3_octet_string_value_create();
    for (size_t i = 0; i < strlen(text); ++i) {
        dnp3_octet_string_value_add(octet_string, (uint8_t)text[i]);
    }
    dnp3_database_update_octet_string(db, 7, octet_string, dnp3_update_options_detect_event());
    dnp3_octet_string_value_destroy(octet_string);
}

void run_transaction(dnp3_outstation_t *outstation, void (*execute)(dnp3_database_t *, void *), void *context)
{
    dnp3_database_transaction_t transaction = {
        .execute = execute,
        .on_destroy = NULL,
        .ctx = context,
    };
    dnp3_outstation_transaction(outstation, transaction);
}

// current time in milliseconds since the UNIX epoch
dnp3_timestamp_t now(void)
{
    struct timespec ts;
    timespec_get(&ts, TIME_UTC);
    uint64_t milliseconds = (uint64_t)ts.tv_sec * 1000 + (uint64_t)ts.tv_nsec / 1000000;
    return dnp3_timestamp_synchronized_timestamp(milliseconds);
}

// read one line without its line ending, returns false at the end of input
bool read_line(char *line, size_t size)
{
    if (fgets(line, (int)size, stdin) == NULL) {
        return false;
    }
    size_t length = strcspn(line, "\r\n");
    if (line[length] == '\0' && !feof(stdin)) {
        // the line didn't fit: discard the rest of it
        int c = getchar();
        while (c != '\n' && c != EOF) {
            c = getchar();
        }
    }
    line[length] = '\0';
    return true;
}

void print_usage(void)
{
    printf("usage: outstation_example <tcp-server|tcp-client|udp|serial|tls-ca|tls-self-signed> [endpoint|serial-port]\n");
}
