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
#define MASTER_ADDRESS 1
#define OUTSTATION_ADDRESS 1024

// default endpoints and serial port, the optional command line argument replaces them
#define TCP_ENDPOINT "127.0.0.1:20000"
#define TLS_ENDPOINT "127.0.0.1:20001"
#define UDP_LOCAL_ENDPOINT "127.0.0.1:20001"
#define UDP_REMOTE_ENDPOINT "127.0.0.1:20000"
#define SERIAL_PORT "/dev/ttyS0"

// TLS files, relative to the repository root
#define TLS_DOMAIN "test.com"
#define CA_CHAIN_CA_CERT "./certs/ca_chain/ca_cert.pem"
#define CA_CHAIN_ENTITY_CERT "./certs/ca_chain/entity1_cert.pem"
#define CA_CHAIN_ENTITY_KEY "./certs/ca_chain/entity1_key.pem"
#define SELF_SIGNED_PEER_CERT "./certs/self_signed/entity2_cert.pem"
#define SELF_SIGNED_ENTITY_CERT "./certs/self_signed/entity1_cert.pem"
#define SELF_SIGNED_ENTITY_KEY "./certs/self_signed/entity1_key.pem"

int run_transport(dnp3_runtime_t *runtime, const char *transport, const char *argument, dnp3_master_channel_t **channel);
dnp3_master_channel_t *create_tcp_channel(dnp3_runtime_t *runtime, const char *endpoint);
dnp3_master_channel_t *create_udp_channel(dnp3_runtime_t *runtime, const char *local_endpoint);
dnp3_master_channel_t *create_serial_channel(dnp3_runtime_t *runtime, const char *port);
dnp3_master_channel_t *create_tls_channel(dnp3_runtime_t *runtime, const char *endpoint, dnp3_tls_client_config_t tls_config);
dnp3_param_error_t add_association(dnp3_master_channel_t *channel, dnp3_association_id_t *association_id);
dnp3_param_error_t add_udp_association(dnp3_master_channel_t *channel, const char *remote_endpoint, dnp3_association_id_t *association_id);
int run_channel(dnp3_master_channel_t *channel, dnp3_association_id_t association_id);
void run_command(dnp3_master_channel_t *channel, dnp3_association_id_t association_id, dnp3_poll_id_t poll_id, const char *cmd);
void print_help(void);
dnp3_master_channel_config_t get_master_channel_config(void);
dnp3_association_config_t get_association_config(void);
dnp3_tls_client_config_t get_tls_ca_config(void);
dnp3_tls_client_config_t get_tls_self_signed_config(void);
dnp3_logger_t get_logger(void);
dnp3_read_handler_t get_read_handler(void);
dnp3_association_handler_t get_association_handler(void);
dnp3_association_information_t get_association_information(void);
dnp3_client_state_listener_t get_client_state_listener(void);
dnp3_port_state_listener_t get_port_state_listener(void);
void on_read_complete(dnp3_nothing_t nothing, void *context);
void on_read_failure(dnp3_read_error_t error, void *context);
void on_command_complete(dnp3_nothing_t nothing, void *context);
void on_command_failure(dnp3_command_error_t error, void *context);
void on_command_sent(dnp3_nothing_t nothing, void *context);
void on_no_ack_command_failure(dnp3_command_error_t error, void *context);
void on_time_sync_complete(dnp3_nothing_t nothing, void *context);
void on_time_sync_failure(dnp3_time_sync_error_t error, void *context);
void on_empty_response_complete(dnp3_nothing_t nothing, void *context);
void on_empty_response_failure(dnp3_empty_response_error_t error, void *context);
void on_restart_complete(uint64_t delay, void *context);
void on_restart_failure(dnp3_restart_error_t error, void *context);
void on_read_directory_complete(dnp3_file_info_iterator_t *iter, void *context);
void on_file_info_complete(dnp3_file_info_t info, void *context);
void on_file_failure(dnp3_file_error_t error, void *context);
dnp3_file_reader_t get_file_reader(void);
void start_write_file(dnp3_master_channel_t *channel, dnp3_association_id_t association_id);
void on_link_status_complete(dnp3_nothing_t nothing, void *context);
void on_link_status_failure(dnp3_link_status_error_t error, void *context);
void print_outcome(const char *cmd, dnp3_param_error_t err, const char *outcome);
void print_file_info(dnp3_file_info_t info);
uint64_t now_ms(void);
bool read_line(char *line, size_t size);
void print_usage(void);

/*
  Example program using the master API.

  The program initializes a master channel based on the command line argument and then enters a loop
  reading console input allowing the user to perform common tasks interactively.
*/
int main(int argc, char *argv[])
{
    if (argc < 2 || argc > 3) {
        print_usage();
        return EXIT_FAILURE;
    }
    const char *transport = argv[1];
    // replaces the endpoint, the UDP remote endpoint or the serial port
    const char *argument = (argc == 3) ? argv[2] : NULL;

    // ANCHOR: logging_init
    // initialize logging with the default configuration
    dnp3_configure_logging(dnp3_logging_config_init(), get_logger());
    // ANCHOR_END: logging_init

    // ANCHOR: runtime_init
    dnp3_runtime_t *runtime = NULL;
    dnp3_runtime_config_t runtime_config = dnp3_runtime_config_init();
    runtime_config.num_core_threads = 4;
    dnp3_param_error_t err = dnp3_runtime_create(runtime_config, &runtime);
    // ANCHOR_END: runtime_init

    if (err) {
        printf("unable to create runtime: %s\n", dnp3_param_error_to_string(err));
        return EXIT_FAILURE;
    }

    // create a channel based on the command line arguments and run it
    dnp3_master_channel_t *channel = NULL;
    int ret = run_transport(runtime, transport, argument, &channel);

    // ANCHOR: runtime_destroy
    // shutting down the runtime first stops every task and callback, including those that use the channel
    dnp3_runtime_destroy(runtime);
    dnp3_master_channel_destroy(channel);
    // ANCHOR_END: runtime_destroy

    return ret;
}

// create the channel and association for the transport, then run the console: the caller destroys the channel
int run_transport(dnp3_runtime_t *runtime, const char *transport, const char *argument, dnp3_master_channel_t **channel)
{
    dnp3_association_id_t association_id;
    dnp3_param_error_t err = DNP3_PARAM_ERROR_OK;

    if (strcmp(transport, "tcp-client") == 0) {
        *channel = create_tcp_channel(runtime, argument ? argument : TCP_ENDPOINT);
        err = add_association(*channel, &association_id);
    }
    else if (strcmp(transport, "udp") == 0) {
        *channel = create_udp_channel(runtime, UDP_LOCAL_ENDPOINT);
        err = add_udp_association(*channel, argument ? argument : UDP_REMOTE_ENDPOINT, &association_id);
    }
    else if (strcmp(transport, "serial") == 0) {
        *channel = create_serial_channel(runtime, argument ? argument : SERIAL_PORT);
        err = add_association(*channel, &association_id);
    }
    else if (strcmp(transport, "tls-ca") == 0) {
        *channel = create_tls_channel(runtime, argument ? argument : TLS_ENDPOINT, get_tls_ca_config());
        err = add_association(*channel, &association_id);
    }
    else if (strcmp(transport, "tls-self-signed") == 0) {
        *channel = create_tls_channel(runtime, argument ? argument : TLS_ENDPOINT, get_tls_self_signed_config());
        err = add_association(*channel, &association_id);
    }
    else {
        print_usage();
        return EXIT_FAILURE;
    }

    int ret = EXIT_FAILURE;
    if (*channel == NULL) {
        // the create function printed the error
    }
    else if (err) {
        printf("unable to add association: %s\n", dnp3_param_error_to_string(err));
    }
    else {
        ret = run_channel(*channel, association_id);
    }
    return ret;
}

dnp3_master_channel_t *create_tcp_channel(dnp3_runtime_t *runtime, const char *endpoint)
{
    // ANCHOR: create_tcp_channel
    dnp3_master_channel_t *channel = NULL;
    dnp3_endpoint_list_t *endpoints = dnp3_endpoint_list_create(endpoint);
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
    // ANCHOR_END: create_tcp_channel

    if (err) {
        printf("unable to create TCP channel: %s\n", dnp3_param_error_to_string(err));
    }
    return channel;
}

dnp3_master_channel_t *create_udp_channel(dnp3_runtime_t *runtime, const char *local_endpoint)
{
    // ANCHOR: create_udp_channel
    dnp3_master_channel_t *channel = NULL;
    dnp3_param_error_t err = dnp3_master_channel_create_udp(
        runtime,
        get_master_channel_config(),
        local_endpoint,
        DNP3_LINK_READ_MODE_DATAGRAM,
        5000, // retry binding the socket every 5 seconds
        &channel
    );
    // ANCHOR_END: create_udp_channel

    if (err) {
        printf("unable to create UDP channel: %s\n", dnp3_param_error_to_string(err));
    }
    return channel;
}

dnp3_master_channel_t *create_serial_channel(dnp3_runtime_t *runtime, const char *port)
{
    // ANCHOR: create_serial_channel
    dnp3_master_channel_t *channel = NULL;
    dnp3_param_error_t err = dnp3_master_channel_create_serial(
        runtime,
        get_master_channel_config(),
        port,
        dnp3_serial_settings_init(), // 9600 8N1, no flow control
        1000,                        // retry opening the port every second
        get_port_state_listener(),
        &channel
    );
    // ANCHOR_END: create_serial_channel

    if (err) {
        printf("unable to create serial channel: %s\n", dnp3_param_error_to_string(err));
    }
    return channel;
}

dnp3_master_channel_t *create_tls_channel(dnp3_runtime_t *runtime, const char *endpoint, dnp3_tls_client_config_t tls_config)
{
    // ANCHOR: create_tls_channel
    dnp3_master_channel_t *channel = NULL;
    dnp3_endpoint_list_t *endpoints = dnp3_endpoint_list_create(endpoint);
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
    // ANCHOR_END: create_tls_channel

    if (err) {
        printf("unable to create TLS channel: %s\n", dnp3_param_error_to_string(err));
    }
    return channel;
}

dnp3_param_error_t add_association(dnp3_master_channel_t *channel, dnp3_association_id_t *association_id)
{
    // ANCHOR: association_create
    dnp3_param_error_t err = dnp3_master_channel_add_association(
        channel,
        OUTSTATION_ADDRESS,
        get_association_config(),
        get_read_handler(),
        get_association_handler(),
        get_association_information(),
        association_id
    );
    // ANCHOR_END: association_create
    return err;
}

dnp3_param_error_t add_udp_association(dnp3_master_channel_t *channel, const char *remote_endpoint, dnp3_association_id_t *association_id)
{
    // ANCHOR: association_create_udp
    dnp3_param_error_t err = dnp3_master_channel_add_udp_association(
        channel,
        OUTSTATION_ADDRESS,
        remote_endpoint,
        get_association_config(),
        get_read_handler(),
        get_association_handler(),
        get_association_information(),
        association_id
    );
    // ANCHOR_END: association_create_udp
    return err;
}

int run_channel(dnp3_master_channel_t *channel, dnp3_association_id_t association_id)
{
    // create an event poll
    // ANCHOR: add_poll
    dnp3_request_t *poll_request = dnp3_request_new_class(false, true, true, true);
    dnp3_poll_id_t poll_id;
    dnp3_param_error_t err = dnp3_master_channel_add_poll(channel, association_id, poll_request, 5000, &poll_id);
    dnp3_request_destroy(poll_request);
    // ANCHOR_END: add_poll
    if (err) {
        printf("unable to add poll: %s\n", dnp3_param_error_to_string(err));
        return EXIT_FAILURE;
    }

    // enable communications
    err = dnp3_master_channel_enable(channel);
    if (err) {
        printf("unable to enable channel: %s\n", dnp3_param_error_to_string(err));
        return EXIT_FAILURE;
    }

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
        else {
            run_command(channel, association_id, poll_id, line);
        }
    }
    return EXIT_SUCCESS;
}

// start one command: the callbacks print the result of the asynchronous ones
void run_command(dnp3_master_channel_t *channel, dnp3_association_id_t association_id, dnp3_poll_id_t poll_id, const char *cmd)
{
    if (strcmp(cmd, "enable") == 0) {
        print_outcome("enable", dnp3_master_channel_enable(channel), "success");
    }
    else if (strcmp(cmd, "disable") == 0) {
        print_outcome("disable", dnp3_master_channel_disable(channel), "success");
    }
    else if (strcmp(cmd, "dln") == 0) {
        print_outcome("dln", dnp3_master_channel_set_decode_level(channel, dnp3_decode_level_nothing()), "success");
    }
    else if (strcmp(cmd, "dlv") == 0) {
        dnp3_decode_level_t level = dnp3_decode_level_nothing();
        level.application = DNP3_APP_DECODE_LEVEL_OBJECT_VALUES;
        print_outcome("dlv", dnp3_master_channel_set_decode_level(channel, level), "success");
    }
    else if (strcmp(cmd, "rao") == 0) {
        dnp3_request_t *request = dnp3_request_create();
        dnp3_request_add_all_objects_header(request, DNP3_VARIATION_GROUP40_VAR0);
        dnp3_read_task_callback_t cb = {
            .on_complete = &on_read_complete,
            .on_failure = &on_read_failure,
            .on_destroy = NULL,
            .ctx = "rao",
        };
        dnp3_param_error_t err = dnp3_master_channel_read(channel, association_id, request, cb);
        dnp3_request_destroy(request);
        print_outcome("rao", err, NULL);
    }
    else if (strcmp(cmd, "rmo") == 0) {
        dnp3_request_t *request = dnp3_request_create();
        dnp3_request_add_all_objects_header(request, DNP3_VARIATION_GROUP10_VAR0);
        dnp3_request_add_all_objects_header(request, DNP3_VARIATION_GROUP40_VAR0);
        dnp3_read_task_callback_t cb = {
            .on_complete = &on_read_complete,
            .on_failure = &on_read_failure,
            .on_destroy = NULL,
            .ctx = "rmo",
        };
        dnp3_param_error_t err = dnp3_master_channel_read(channel, association_id, request, cb);
        dnp3_request_destroy(request);
        print_outcome("rmo", err, NULL);
    }
    else if (strcmp(cmd, "rco") == 0) {
        dnp3_request_t *request = dnp3_request_create();
        dnp3_request_add_all_objects_header(request, DNP3_VARIATION_GROUP20_VAR0);
        dnp3_request_add_all_objects_header(request, DNP3_VARIATION_GROUP21_VAR0);
        dnp3_read_task_callback_t cb = {
            .on_complete = &on_read_complete,
            .on_failure = &on_read_failure,
            .on_destroy = NULL,
            .ctx = "rco",
        };
        dnp3_param_error_t err = dnp3_master_channel_read(channel, association_id, request, cb);
        dnp3_request_destroy(request);
        print_outcome("rco", err, NULL);
    }
    else if (strcmp(cmd, "cmd") == 0) {
        // ANCHOR: assoc_control
        dnp3_command_set_t *commands = dnp3_command_set_create();
        dnp3_group12_var1_t g12v1 = dnp3_group12_var1_init(dnp3_control_code_init(DNP3_TRIP_CLOSE_CODE_NUL, false, DNP3_OP_TYPE_LATCH_ON), 1, 1000, 1000);
        dnp3_command_set_add_g12_v1_u16(commands, 3, g12v1);

        dnp3_command_task_callback_t cb = {
            .on_complete = &on_command_complete,
            .on_failure = &on_command_failure,
            .on_destroy = NULL,
            .ctx = "cmd",
        };

        dnp3_param_error_t err = dnp3_master_channel_operate(channel, association_id, DNP3_COMMAND_MODE_SELECT_BEFORE_OPERATE, commands, cb);
        dnp3_command_set_destroy(commands);
        // ANCHOR_END: assoc_control
        print_outcome("cmd", err, NULL);
    }
    else if (strcmp(cmd, "cna") == 0) {
        // ANCHOR: assoc_control_no_ack
        dnp3_command_set_t *commands = dnp3_command_set_create();
        dnp3_group12_var1_t g12v1 = dnp3_group12_var1_init(dnp3_control_code_init(DNP3_TRIP_CLOSE_CODE_NUL, false, DNP3_OP_TYPE_LATCH_ON), 1, 1000, 1000);
        dnp3_command_set_add_g12_v1_u16(commands, 3, g12v1);

        dnp3_command_task_callback_t cb = {
            .on_complete = &on_command_sent,
            .on_failure = &on_no_ack_command_failure,
            .on_destroy = NULL,
            .ctx = "cna",
        };

        dnp3_param_error_t err = dnp3_master_channel_operate_no_ack(channel, association_id, commands, cb);
        dnp3_command_set_destroy(commands);
        // ANCHOR_END: assoc_control_no_ack
        print_outcome("cna", err, NULL);
    }
    else if (strcmp(cmd, "evt") == 0) {
        // the poll runs later, and its data goes to the read handler
        print_outcome("evt", dnp3_master_channel_demand_poll(channel, poll_id), "poll demanded");
    }
    else if (strcmp(cmd, "lts") == 0) {
        dnp3_time_sync_task_callback_t cb = {
            .on_complete = &on_time_sync_complete,
            .on_failure = &on_time_sync_failure,
            .on_destroy = NULL,
            .ctx = "lts",
        };
        print_outcome("lts", dnp3_master_channel_synchronize_time(channel, association_id, DNP3_TIME_SYNC_MODE_LAN, cb), NULL);
    }
    else if (strcmp(cmd, "nts") == 0) {
        dnp3_time_sync_task_callback_t cb = {
            .on_complete = &on_time_sync_complete,
            .on_failure = &on_time_sync_failure,
            .on_destroy = NULL,
            .ctx = "nts",
        };
        print_outcome("nts", dnp3_master_channel_synchronize_time(channel, association_id, DNP3_TIME_SYNC_MODE_NON_LAN, cb), NULL);
    }
    else if (strcmp(cmd, "wad") == 0) {
        // ANCHOR: write_dead_bands
        dnp3_write_dead_band_request_t *request = dnp3_write_dead_band_request_create();
        dnp3_write_dead_band_request_add_g34v1_u8(request, 3, 5);
        dnp3_write_dead_band_request_add_g34v3_u16(request, 4, 2.5f);

        dnp3_empty_response_callback_t cb = {
            .on_complete = &on_empty_response_complete,
            .on_failure = &on_empty_response_failure,
            .on_destroy = NULL,
            .ctx = "wad",
        };

        dnp3_param_error_t err = dnp3_master_channel_write_dead_bands(channel, association_id, request, cb);
        dnp3_write_dead_band_request_destroy(request);
        // ANCHOR_END: write_dead_bands
        print_outcome("wad", err, NULL);
    }
    else if (strcmp(cmd, "ifz") == 0) {
        // copy every counter to its frozen counter
        dnp3_request_t *request = dnp3_request_create();
        dnp3_request_add_all_objects_header(request, DNP3_VARIATION_GROUP20_VAR0);
        dnp3_empty_response_callback_t cb = {
            .on_complete = &on_empty_response_complete,
            .on_failure = &on_empty_response_failure,
            .on_destroy = NULL,
            .ctx = "ifz",
        };
        dnp3_param_error_t err =
            dnp3_master_channel_send_and_expect_empty_response(channel, association_id, DNP3_FUNCTION_CODE_IMMEDIATE_FREEZE, request, cb);
        dnp3_request_destroy(request);
        print_outcome("ifz", err, NULL);
    }
    else if (strcmp(cmd, "fcl") == 0) {
        // copy every counter to its frozen counter, then clear the counter
        dnp3_request_t *request = dnp3_request_create();
        dnp3_request_add_all_objects_header(request, DNP3_VARIATION_GROUP20_VAR0);
        dnp3_empty_response_callback_t cb = {
            .on_complete = &on_empty_response_complete,
            .on_failure = &on_empty_response_failure,
            .on_destroy = NULL,
            .ctx = "fcl",
        };
        dnp3_param_error_t err =
            dnp3_master_channel_send_and_expect_empty_response(channel, association_id, DNP3_FUNCTION_CODE_FREEZE_CLEAR, request, cb);
        dnp3_request_destroy(request);
        print_outcome("fcl", err, NULL);
    }
    else if (strcmp(cmd, "fat") == 0) {
        // ANCHOR: freeze_at_time
        dnp3_request_t *request = dnp3_request_create();
        // freeze all the counters once per day relative to the beginning of the current hour
        dnp3_request_add_time_and_interval(request, 0, 86400000);
        // apply this schedule to all counters
        dnp3_request_add_all_objects_header(request, DNP3_VARIATION_GROUP20_VAR0);

        dnp3_empty_response_callback_t cb = {
            .on_complete = &on_empty_response_complete,
            .on_failure = &on_empty_response_failure,
            .on_destroy = NULL,
            .ctx = "fat",
        };

        dnp3_param_error_t err =
            dnp3_master_channel_send_and_expect_empty_response(channel, association_id, DNP3_FUNCTION_CODE_FREEZE_AT_TIME, request, cb);
        dnp3_request_destroy(request);
        // ANCHOR_END: freeze_at_time
        print_outcome("fat", err, NULL);
    }
    else if (strcmp(cmd, "rda") == 0) {
        // ANCHOR: read_attributes
        dnp3_request_t *request = dnp3_request_create();
        dnp3_request_add_specific_attribute(request, DNP3_ATTRIBUTE_VARIATIONS_ALL_ATTRIBUTES_REQUEST, 0);

        dnp3_read_task_callback_t cb = {
            .on_complete = &on_read_complete,
            .on_failure = &on_read_failure,
            .on_destroy = NULL,
            .ctx = "rda",
        };

        dnp3_param_error_t err = dnp3_master_channel_read(channel, association_id, request, cb);
        dnp3_request_destroy(request);
        // ANCHOR_END: read_attributes
        print_outcome("rda", err, NULL);
    }
    else if (strcmp(cmd, "wda") == 0) {
        // ANCHOR: write_attribute
        dnp3_request_t *request = dnp3_request_create();
        dnp3_request_add_string_attribute(request, DNP3_ATTRIBUTE_VARIATIONS_USER_ASSIGNED_LOCATION, 0, "Mt. Olympus");

        dnp3_empty_response_callback_t cb = {
            .on_complete = &on_empty_response_complete,
            .on_failure = &on_empty_response_failure,
            .on_destroy = NULL,
            .ctx = "wda",
        };

        dnp3_param_error_t err = dnp3_master_channel_send_and_expect_empty_response(channel, association_id, DNP3_FUNCTION_CODE_WRITE, request, cb);
        dnp3_request_destroy(request);
        // ANCHOR_END: write_attribute
        print_outcome("wda", err, NULL);
    }
    else if (strcmp(cmd, "ral") == 0) {
        dnp3_request_t *request = dnp3_request_create();
        dnp3_request_add_specific_attribute(request, DNP3_ATTRIBUTE_VARIATIONS_LIST_OF_VARIATIONS, 0);
        dnp3_read_task_callback_t cb = {
            .on_complete = &on_read_complete,
            .on_failure = &on_read_failure,
            .on_destroy = NULL,
            .ctx = "ral",
        };
        dnp3_param_error_t err = dnp3_master_channel_read(channel, association_id, request, cb);
        dnp3_request_destroy(request);
        print_outcome("ral", err, NULL);
    }
    else if (strcmp(cmd, "crt") == 0) {
        dnp3_restart_task_callback_t cb = {
            .on_complete = &on_restart_complete,
            .on_failure = &on_restart_failure,
            .on_destroy = NULL,
            .ctx = "crt",
        };
        print_outcome("crt", dnp3_master_channel_cold_restart(channel, association_id, cb), NULL);
    }
    else if (strcmp(cmd, "wrt") == 0) {
        dnp3_restart_task_callback_t cb = {
            .on_complete = &on_restart_complete,
            .on_failure = &on_restart_failure,
            .on_destroy = NULL,
            .ctx = "wrt",
        };
        print_outcome("wrt", dnp3_master_channel_warm_restart(channel, association_id, cb), NULL);
    }
    else if (strcmp(cmd, "rd") == 0) {
        // ANCHOR: read_directory
        dnp3_read_directory_callback_t cb = {
            .on_complete = &on_read_directory_complete,
            .on_failure = &on_file_failure,
            .on_destroy = NULL,
            .ctx = "rd",
        };
        dnp3_param_error_t err = dnp3_master_channel_read_directory(channel, association_id, ".", dnp3_dir_read_config_defaults(), cb);
        // ANCHOR_END: read_directory
        print_outcome("rd", err, NULL);
    }
    else if (strcmp(cmd, "gfi") == 0) {
        // ANCHOR: get_file_info
        dnp3_file_info_callback_t cb = {
            .on_complete = &on_file_info_complete,
            .on_failure = &on_file_failure,
            .on_destroy = NULL,
            .ctx = "gfi",
        };
        dnp3_param_error_t err = dnp3_master_channel_get_file_info(channel, association_id, ".", cb);
        // ANCHOR_END: get_file_info
        print_outcome("gfi", err, NULL);
    }
    else if (strcmp(cmd, "rf") == 0) {
        // ANCHOR: read_file
        dnp3_param_error_t err = dnp3_master_channel_read_file(
            channel,
            association_id,
            ".", // this reads the root "directory" file but doesn't parse it
            dnp3_file_read_config_defaults(),
            get_file_reader()
        );
        // ANCHOR_END: read_file
        print_outcome("rf", err, "started (the file reader reports the outcome)");
    }
    else if (strcmp(cmd, "wf") == 0) {
        start_write_file(channel, association_id);
    }
    else if (strcmp(cmd, "lsr") == 0) {
        dnp3_link_status_callback_t cb = {
            .on_complete = &on_link_status_complete,
            .on_failure = &on_link_status_failure,
            .on_destroy = NULL,
            .ctx = "lsr",
        };
        print_outcome("lsr", dnp3_master_channel_check_link_status(channel, association_id, cb), NULL);
    }
    else {
        printf("%s: unknown command, type help for a list\n", cmd);
    }
}

void print_help(void)
{
    printf("Commands:\n");
    printf("  help - print this list\n");
    printf("  x - exit\n");
    printf("  enable / disable - enable or disable the channel\n");
    printf("  dln / dlv - decode nothing / decode object values\n");
    printf("  rao - read analog output statuses\n");
    printf("  rmo - read binary and analog output statuses\n");
    printf("  rco - read counters and frozen counters\n");
    printf("  cmd - select and operate binary output 3 (latch on)\n");
    printf("  cna - operate binary output 3 without an answer\n");
    printf("  evt - demand the event poll\n");
    printf("  lts / nts - synchronize the time (LAN / non-LAN)\n");
    printf("  wad - write analog input deadbands\n");
    printf("  ifz - freeze all counters\n");
    printf("  fcl - freeze and clear all counters\n");
    printf("  fat - freeze all counters once a day\n");
    printf("  rda - read all device attributes\n");
    printf("  wda - write the user-assigned location attribute\n");
    printf("  ral - read the list of attribute variations\n");
    printf("  crt / wrt - cold / warm restart\n");
    printf("  rd - read the root directory\n");
    printf("  gfi - get information about the root directory\n");
    printf("  rf - read the root directory as a file\n");
    printf("  wf - write hello_world.txt\n");
    printf("  lsr - check the link status\n");
}

// ANCHOR: master_channel_config
dnp3_master_channel_config_t get_master_channel_config(void)
{
    dnp3_master_channel_config_t config = dnp3_master_channel_config_init(MASTER_ADDRESS);
    config.decode_level.application = DNP3_APP_DECODE_LEVEL_OBJECT_VALUES;
    return config;
}
// ANCHOR_END: master_channel_config

// ANCHOR: association_config
dnp3_association_config_t get_association_config(void)
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

dnp3_tls_client_config_t get_tls_ca_config(void)
{
    // ANCHOR: tls_ca_chain_config
    dnp3_tls_client_config_t config = dnp3_tls_client_config_init(
        TLS_DOMAIN,
        CA_CHAIN_CA_CERT,
        CA_CHAIN_ENTITY_CERT,
        CA_CHAIN_ENTITY_KEY,
        "" // no password
    );
    // ANCHOR_END: tls_ca_chain_config
    return config;
}

dnp3_tls_client_config_t get_tls_self_signed_config(void)
{
    // ANCHOR: tls_self_signed_config
    dnp3_tls_client_config_t config = dnp3_tls_client_config_init(
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

// ANCHOR: logging_interface
// callback which will receive log messages
void on_log_message(dnp3_log_level_t level, const char *msg, void *context) { printf("%s", msg); }

dnp3_logger_t get_logger(void)
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
// ANCHOR_END: logging_interface

// ANCHOR: read_handler
// print helpers, defined after the callbacks
void print_header(const char *name, dnp3_header_info_t info);
void print_flags_and_time(dnp3_flags_t flags, dnp3_timestamp_t time);
void print_time(dnp3_timestamp_t time);
void print_bytes(dnp3_byte_iterator_t *bytes);

void begin_fragment(dnp3_read_type_t read_type, dnp3_response_header_t header, void *context)
{
    printf("begin fragment: %s (broadcast: %s)\n", dnp3_read_type_to_string(read_type), header.iin.iin1.broadcast ? "true" : "false");
}

void end_fragment(dnp3_read_type_t read_type, dnp3_response_header_t header, void *context) { printf("end fragment\n"); }

void handle_binary_input(dnp3_header_info_t info, dnp3_binary_input_iterator_t *it, void *context)
{
    print_header("Binary inputs", info);
    dnp3_binary_input_t *value = NULL;
    while ((value = dnp3_binary_input_iterator_next(it))) {
        printf("BI %u: value=%s", value->index, value->value ? "true" : "false");
        print_flags_and_time(value->flags, value->time);
    }
}

void handle_double_bit_binary_input(dnp3_header_info_t info, dnp3_double_bit_binary_input_iterator_t *it, void *context)
{
    print_header("Double-bit binary inputs", info);
    dnp3_double_bit_binary_input_t *value = NULL;
    while ((value = dnp3_double_bit_binary_input_iterator_next(it))) {
        printf("DBBI %u: value=%s", value->index, dnp3_double_bit_to_string(value->value));
        print_flags_and_time(value->flags, value->time);
    }
}

void handle_binary_output_status(dnp3_header_info_t info, dnp3_binary_output_status_iterator_t *it, void *context)
{
    print_header("Binary output statuses", info);
    dnp3_binary_output_status_t *value = NULL;
    while ((value = dnp3_binary_output_status_iterator_next(it))) {
        printf("BOS %u: value=%s", value->index, value->value ? "true" : "false");
        print_flags_and_time(value->flags, value->time);
    }
}

void handle_counter(dnp3_header_info_t info, dnp3_counter_iterator_t *it, void *context)
{
    print_header("Counters", info);
    dnp3_counter_t *value = NULL;
    while ((value = dnp3_counter_iterator_next(it))) {
        printf("C %u: value=%u", value->index, value->value);
        print_flags_and_time(value->flags, value->time);
    }
}

void handle_frozen_counter(dnp3_header_info_t info, dnp3_frozen_counter_iterator_t *it, void *context)
{
    print_header("Frozen counters", info);
    dnp3_frozen_counter_t *value = NULL;
    while ((value = dnp3_frozen_counter_iterator_next(it))) {
        printf("FC %u: value=%u", value->index, value->value);
        print_flags_and_time(value->flags, value->time);
    }
}

void handle_analog_input(dnp3_header_info_t info, dnp3_analog_input_iterator_t *it, void *context)
{
    print_header("Analog inputs", info);
    dnp3_analog_input_t *value = NULL;
    while ((value = dnp3_analog_input_iterator_next(it))) {
        printf("AI %u: value=%.15g", value->index, value->value);
        print_flags_and_time(value->flags, value->time);
    }
}

void handle_frozen_analog_input(dnp3_header_info_t info, dnp3_frozen_analog_input_iterator_t *it, void *context)
{
    print_header("Frozen analog inputs", info);
    dnp3_frozen_analog_input_t *value = NULL;
    while ((value = dnp3_frozen_analog_input_iterator_next(it))) {
        printf("FAI %u: value=%.15g", value->index, value->value);
        print_flags_and_time(value->flags, value->time);
    }
}

void handle_analog_input_dead_band(dnp3_header_info_t info, dnp3_analog_input_dead_band_iterator_t *it, void *context)
{
    print_header("Analog input deadbands", info);
    dnp3_analog_input_dead_band_t *value = NULL;
    while ((value = dnp3_analog_input_dead_band_iterator_next(it))) {
        printf("AIDB %u: value=%.15g\n", value->index, value->value);
    }
}

void handle_analog_output_status(dnp3_header_info_t info, dnp3_analog_output_status_iterator_t *it, void *context)
{
    print_header("Analog output statuses", info);
    dnp3_analog_output_status_t *value = NULL;
    while ((value = dnp3_analog_output_status_iterator_next(it))) {
        printf("AOS %u: value=%.15g", value->index, value->value);
        print_flags_and_time(value->flags, value->time);
    }
}

void handle_binary_output_command_event(dnp3_header_info_t info, dnp3_binary_output_command_event_iterator_t *it, void *context)
{
    print_header("Binary output command events", info);
    dnp3_binary_output_command_event_t *value = NULL;
    while ((value = dnp3_binary_output_command_event_iterator_next(it))) {
        printf("BOCE %u: commanded state=%s status=%s time=", value->index, value->commanded_state ? "true" : "false",
               dnp3_command_status_to_string(value->status));
        print_time(value->time);
        printf("\n");
    }
}

void handle_analog_output_command_event(dnp3_header_info_t info, dnp3_analog_output_command_event_iterator_t *it, void *context)
{
    print_header("Analog output command events", info);
    dnp3_analog_output_command_event_t *value = NULL;
    while ((value = dnp3_analog_output_command_event_iterator_next(it))) {
        printf("AOCE %u: commanded value=%.15g status=%s time=", value->index, value->commanded_value, dnp3_command_status_to_string(value->status));
        print_time(value->time);
        printf("\n");
    }
}

void handle_unsigned_integer(dnp3_header_info_t info, dnp3_unsigned_integer_iterator_t *it, void *context)
{
    print_header("Unsigned integers", info);
    dnp3_unsigned_integer_t *value = NULL;
    while ((value = dnp3_unsigned_integer_iterator_next(it))) {
        printf("UI %u: value=%u\n", value->index, value->value);
    }
}

void handle_octet_string(dnp3_header_info_t info, dnp3_octet_string_iterator_t *it, void *context)
{
    print_header("Octet strings", info);
    dnp3_octet_string_t *value = NULL;
    while ((value = dnp3_octet_string_iterator_next(it))) {
        printf("OS %u: value=", value->index);
        print_bytes(value->value);
        printf("\n");
    }
}

void handle_abs_time(dnp3_header_info_t info, dnp3_timestamp_t time, void *context)
{
    print_header("Absolute time", info);
    printf("time=%" PRIu64 "\n", time.value);
}

void handle_string_attr(dnp3_header_info_t info, dnp3_string_attr_t attr, uint8_t set, uint8_t variation, const char *value, void *context)
{
    print_header("Device attributes", info);
    printf("attribute set=%u variation=%u value=%s\n", set, variation, value);
}

void handle_variation_list_attr(dnp3_header_info_t info, dnp3_variation_list_attr_t attr, uint8_t set, uint8_t variation,
                                dnp3_attr_item_iter_t *items, void *context)
{
    print_header("Device attributes", info);
    printf("attribute set=%u variation=%u value=[", set, variation);
    dnp3_attr_item_t *item = dnp3_attr_item_iter_next(items);
    while (item) {
        printf("%u", item->variation);
        item = dnp3_attr_item_iter_next(items);
        printf("%s", item ? ", " : "");
    }
    printf("]\n");
}

void handle_uint_attr(dnp3_header_info_t info, dnp3_uint_attr_t attr, uint8_t set, uint8_t variation, uint32_t value, void *context)
{
    print_header("Device attributes", info);
    printf("attribute set=%u variation=%u value=%u\n", set, variation, value);
}

void handle_bool_attr(dnp3_header_info_t info, dnp3_bool_attr_t attr, uint8_t set, uint8_t variation, bool value, void *context)
{
    print_header("Device attributes", info);
    printf("attribute set=%u variation=%u value=%s\n", set, variation, value ? "true" : "false");
}

void handle_int_attr(dnp3_header_info_t info, dnp3_int_attr_t attr, uint8_t set, uint8_t variation, int32_t value, void *context)
{
    print_header("Device attributes", info);
    printf("attribute set=%u variation=%u value=%d\n", set, variation, value);
}

void handle_time_attr(dnp3_header_info_t info, dnp3_time_attr_t attr, uint8_t set, uint8_t variation, uint64_t value, void *context)
{
    print_header("Device attributes", info);
    printf("attribute set=%u variation=%u value=%" PRIu64 "\n", set, variation, value);
}

void handle_float_attr(dnp3_header_info_t info, dnp3_float_attr_t attr, uint8_t set, uint8_t variation, double value, void *context)
{
    print_header("Device attributes", info);
    printf("attribute set=%u variation=%u value=%.15g\n", set, variation, value);
}

void handle_octet_string_attr(dnp3_header_info_t info, dnp3_octet_string_attr_t attr, uint8_t set, uint8_t variation, dnp3_byte_iterator_t *value,
                              void *context)
{
    print_header("Device attributes", info);
    printf("attribute set=%u variation=%u value=", set, variation);
    print_bytes(value);
    printf("\n");
}

void handle_bit_string_attr(dnp3_header_info_t info, dnp3_bit_string_attr_t attr, uint8_t set, uint8_t variation, dnp3_byte_iterator_t *value,
                            void *context)
{
    print_header("Device attributes", info);
    printf("attribute set=%u variation=%u value=", set, variation);
    print_bytes(value);
    printf("\n");
}

dnp3_read_handler_t get_read_handler(void)
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
        .handle_frozen_analog_input = &handle_frozen_analog_input,
        .handle_analog_output_status = &handle_analog_output_status,
        .handle_binary_output_command_event = &handle_binary_output_command_event,
        .handle_analog_output_command_event = &handle_analog_output_command_event,
        .handle_unsigned_integer = &handle_unsigned_integer,
        .handle_analog_input_dead_band = &handle_analog_input_dead_band,
        .handle_octet_string = &handle_octet_string,
        .handle_abs_time = &handle_abs_time,
        .handle_string_attr = &handle_string_attr,
        .handle_variation_list_attr = &handle_variation_list_attr,
        .handle_uint_attr = &handle_uint_attr,
        .handle_bool_attr = &handle_bool_attr,
        .handle_int_attr = &handle_int_attr,
        .handle_time_attr = &handle_time_attr,
        .handle_float_attr = &handle_float_attr,
        .handle_octet_string_attr = &handle_octet_string_attr,
        .handle_bit_string_attr = &handle_bit_string_attr,
        .on_destroy = NULL,
        .ctx = NULL,
    };
}

void print_header(const char *name, dnp3_header_info_t info)
{
    printf("%s (%s, %s)\n", name, dnp3_qualifier_code_to_string(info.qualifier), dnp3_variation_to_string(info.variation));
}

// ends the line of a measurement with its flags and time
void print_flags_and_time(dnp3_flags_t flags, dnp3_timestamp_t time)
{
    printf(" flags=0x%02X time=", flags.value);
    print_time(time);
    printf("\n");
}

void print_time(dnp3_timestamp_t time)
{
    if (time.quality == DNP3_TIME_QUALITY_SYNCHRONIZED_TIME) {
        printf("%" PRIu64 " (synchronized)", time.value);
    }
    else if (time.quality == DNP3_TIME_QUALITY_UNSYNCHRONIZED_TIME) {
        printf("%" PRIu64 " (unsynchronized)", time.value);
    }
    else {
        printf("none");
    }
}

// prints the bytes as a list of hex values
void print_bytes(dnp3_byte_iterator_t *bytes)
{
    printf("[");
    uint8_t *byte = dnp3_byte_iterator_next(bytes);
    while (byte) {
        printf("%02X", *byte);
        byte = dnp3_byte_iterator_next(bytes);
        printf("%s", byte ? ", " : "");
    }
    printf("]");
}
// ANCHOR_END: read_handler

// ANCHOR: association_handler
dnp3_utc_timestamp_t get_system_time(void *context) { return dnp3_utc_timestamp_valid(now_ms()); }

dnp3_association_handler_t get_association_handler(void)
{
    return (dnp3_association_handler_t){
        .get_current_time = &get_system_time,
        .on_destroy = NULL,
        .ctx = NULL,
    };
}
// ANCHOR_END: association_handler

// ANCHOR: association_information
dnp3_association_information_t get_association_information(void)
{
    // NULL callbacks keep their default: do nothing
    return (dnp3_association_information_t){
        .on_destroy = NULL,
        .ctx = NULL,
    };
}
// ANCHOR_END: association_information

// listeners that print each state change of a client connection or serial port
void on_client_state_change(dnp3_client_state_t state, void *context) { printf("client state: %s\n", dnp3_client_state_to_string(state)); }

void on_port_state_change(dnp3_port_state_t state, void *context) { printf("port state: %s\n", dnp3_port_state_to_string(state)); }

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

// The completion callbacks below receive the name of their command as the context

void on_read_complete(dnp3_nothing_t nothing, void *context) { printf("%s: success\n", (const char *)context); }

void on_read_failure(dnp3_read_error_t error, void *context) { printf("%s: %s\n", (const char *)context, dnp3_read_error_to_string(error)); }

// ANCHOR: assoc_control_callback
void on_command_complete(dnp3_nothing_t nothing, void *context) { printf("%s: success\n", (const char *)context); }

void on_command_failure(dnp3_command_error_t error, void *context)
{
    printf("%s: %s\n", (const char *)context, dnp3_command_error_to_string(error));
}
// ANCHOR_END: assoc_control_callback

// ANCHOR: assoc_control_no_ack_callback
void on_command_sent(dnp3_nothing_t nothing, void *context)
{
    // the request was sent: the outstation doesn't answer
    printf("%s: sent\n", (const char *)context);
}

void on_no_ack_command_failure(dnp3_command_error_t error, void *context)
{
    printf("%s: %s\n", (const char *)context, dnp3_command_error_to_string(error));
}
// ANCHOR_END: assoc_control_no_ack_callback

void on_time_sync_complete(dnp3_nothing_t nothing, void *context) { printf("%s: success\n", (const char *)context); }

void on_time_sync_failure(dnp3_time_sync_error_t error, void *context)
{
    printf("%s: %s\n", (const char *)context, dnp3_time_sync_error_to_string(error));
}

void on_empty_response_complete(dnp3_nothing_t nothing, void *context) { printf("%s: success\n", (const char *)context); }

void on_empty_response_failure(dnp3_empty_response_error_t error, void *context)
{
    printf("%s: %s\n", (const char *)context, dnp3_empty_response_error_to_string(error));
}

void on_restart_complete(uint64_t delay, void *context)
{
    printf("restart delay: %" PRIu64 " ms\n", delay);
    printf("%s: success\n", (const char *)context);
}

void on_restart_failure(dnp3_restart_error_t error, void *context) { printf("%s: %s\n", (const char *)context, dnp3_restart_error_to_string(error)); }

// ANCHOR: read_directory_callback
void on_read_directory_complete(dnp3_file_info_iterator_t *iter, void *context)
{
    dnp3_file_info_t *info = NULL;
    while ((info = dnp3_file_info_iterator_next(iter))) {
        print_file_info(*info);
    }
    printf("%s: success\n", (const char *)context);
}
// ANCHOR_END: read_directory_callback

// ANCHOR: file_info_callback
void on_file_info_complete(dnp3_file_info_t info, void *context)
{
    print_file_info(info);
    printf("%s: success\n", (const char *)context);
}

void on_file_failure(dnp3_file_error_t error, void *context) { printf("%s: %s\n", (const char *)context, dnp3_file_error_to_string(error)); }
// ANCHOR_END: file_info_callback

// ANCHOR: file_logger
bool on_file_opened(uint32_t size, void *context)
{
    printf("File opened - size: %u\n", size);
    return true; // continue
}

bool on_file_block_received(uint32_t block_num, dnp3_byte_iterator_t *data, void *context)
{
    uint32_t size = 0;
    while (dnp3_byte_iterator_next(data)) {
        ++size;
    }
    printf("Received block %u with size: %u\n", block_num, size);
    return true; // continue
}

void on_file_aborted(dnp3_file_error_t error, void *context) { printf("rf: aborted: %s\n", dnp3_file_error_to_string(error)); }

void on_file_completed(void *context) { printf("rf: completed\n"); }

dnp3_file_reader_t get_file_reader(void)
{
    return (dnp3_file_reader_t){
        .opened = &on_file_opened,
        .block_received = &on_file_block_received,
        .aborted = &on_file_aborted,
        .completed = &on_file_completed,
        .on_destroy = NULL,
        .ctx = NULL,
    };
}
// ANCHOR_END: file_logger

// ANCHOR: write_file
// what each step of the write needs: each step's callback owns a copy and frees it
typedef struct write_file_t {
    dnp3_master_channel_t *channel;
    dnp3_association_id_t association_id;
    uint32_t handle;
} write_file_t;

write_file_t *new_write_file(dnp3_master_channel_t *channel, dnp3_association_id_t association_id, uint32_t handle)
{
    write_file_t *state = malloc(sizeof(write_file_t));
    if (state == NULL) {
        printf("wf: out of memory\n");
        return NULL;
    }
    state->channel = channel;
    state->association_id = association_id;
    state->handle = handle;
    return state;
}

void on_write_file_failure(dnp3_file_error_t error, void *context) { printf("wf: %s\n", dnp3_file_error_to_string(error)); }

void on_write_file_closed(dnp3_nothing_t nothing, void *context) { printf("wf: success\n"); }

void print_write_file_error(dnp3_param_error_t err)
{
    if (err) {
        printf("wf: %s\n", dnp3_param_error_to_string(err));
    }
}

void on_last_block_written(dnp3_nothing_t nothing, void *context)
{
    write_file_t *state = (write_file_t *)context;
    dnp3_file_operation_callback_t cb = {
        .on_complete = &on_write_file_closed,
        .on_failure = &on_write_file_failure,
        .on_destroy = NULL,
        .ctx = NULL,
    };
    print_write_file_error(dnp3_master_channel_close_file(state->channel, state->association_id, state->handle, cb));
}

// write the 'hello world' line as one block of the file
void write_hello_world_block(write_file_t *state, uint32_t block, bool last, void (*on_written)(dnp3_nothing_t, void *))
{
    write_file_t *next = new_write_file(state->channel, state->association_id, state->handle);
    if (next == NULL) {
        return;
    }
    const char *line = "hello world\n";
    dnp3_byte_collection_t *data = dnp3_byte_collection_create((uint32_t)strlen(line));
    for (size_t i = 0; i < strlen(line); ++i) {
        dnp3_byte_collection_add(data, (uint8_t)line[i]);
    }
    dnp3_file_operation_callback_t cb = {
        .on_complete = on_written,
        .on_failure = &on_write_file_failure,
        .on_destroy = &free,
        .ctx = next,
    };
    print_write_file_error(dnp3_master_channel_write_file_block(state->channel, state->association_id, state->handle, block, last, data, cb));
    dnp3_byte_collection_destroy(data);
}

void on_first_block_written(dnp3_nothing_t nothing, void *context)
{
    write_hello_world_block((write_file_t *)context, 1, true, &on_last_block_written);
}

void on_write_file_opened(dnp3_open_file_t file, void *context)
{
    write_file_t *state = (write_file_t *)context;
    state->handle = file.file_handle;
    // write the 'hello world' line to the file twice
    write_hello_world_block(state, 0, false, &on_first_block_written);
}

// open hello_world.txt for writing: the callbacks above write it and close it
void start_write_file(dnp3_master_channel_t *channel, dnp3_association_id_t association_id)
{
    write_file_t *state = new_write_file(channel, association_id, 0);
    if (state == NULL) {
        return;
    }
    dnp3_file_open_callback_t cb = {
        .on_complete = &on_write_file_opened,
        .on_failure = &on_write_file_failure,
        .on_destroy = &free,
        .ctx = state,
    };
    dnp3_param_error_t err = dnp3_master_channel_open_file(
        channel,
        association_id,
        "hello_world.txt",
        0, // no authentication key
        dnp3_permissions_none(),
        0xFFFFFFFF, // indicate that we don't know the size
        DNP3_FILE_MODE_WRITE,
        512, // maximum block size
        cb
    );
    print_write_file_error(err);
}
// ANCHOR_END: write_file

void on_link_status_complete(dnp3_nothing_t nothing, void *context) { printf("%s: success\n", (const char *)context); }

void on_link_status_failure(dnp3_link_status_error_t error, void *context)
{
    printf("%s: %s\n", (const char *)context, dnp3_link_status_error_to_string(error));
}

// print the outcome of a command, or the error if it didn't start: a NULL outcome leaves it to the callback
void print_outcome(const char *cmd, dnp3_param_error_t err, const char *outcome)
{
    if (err) {
        printf("%s: %s\n", cmd, dnp3_param_error_to_string(err));
    }
    else if (outcome) {
        printf("%s: %s\n", cmd, outcome);
    }
}

void print_file_info(dnp3_file_info_t info)
{
    printf("File name: %s\n", info.file_name);
    printf("     type: %s\n", dnp3_file_type_to_string(info.file_type));
    printf("     size: %u\n", info.size);
    printf("     created: %" PRIu64 "\n", info.time_created);
    printf("     permissions:\n");
    printf("         world: %c%c%c\n", info.permissions.world.read ? 'r' : '-', info.permissions.world.write ? 'w' : '-',
           info.permissions.world.execute ? 'x' : '-');
    printf("         group: %c%c%c\n", info.permissions.group.read ? 'r' : '-', info.permissions.group.write ? 'w' : '-',
           info.permissions.group.execute ? 'x' : '-');
    printf("         owner: %c%c%c\n", info.permissions.owner.read ? 'r' : '-', info.permissions.owner.write ? 'w' : '-',
           info.permissions.owner.execute ? 'x' : '-');
}

// current time in milliseconds since the UNIX epoch
uint64_t now_ms(void)
{
    struct timespec ts;
    timespec_get(&ts, TIME_UTC);
    return (uint64_t)ts.tv_sec * 1000 + (uint64_t)ts.tv_nsec / 1000000;
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
    printf("usage: master_example <tcp-client|udp|serial|tls-ca|tls-self-signed> [endpoint|serial-port]\n");
}
