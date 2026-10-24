// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

// Smoke test of the C bindings: an outstation and a master in one process, talking over TCP
// loopback. It checks that the library loads and that calls, callbacks and async completions
// cross the boundary: a class 0 read returns the outstation's values, and a direct operate
// reaches the outstation's control handler.

#include "dnp3.h"

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>

#ifdef _WIN32
#include <windows.h>
#else
#include <pthread.h>
#include <time.h>
#endif

#define ENDPOINT "127.0.0.1:20101"
#define OUTSTATION_ADDRESS 1024
#define MASTER_ADDRESS 1
#define WAIT_TIMEOUT_MS 10000

typedef enum result_t { RESULT_PENDING, RESULT_SUCCESS, RESULT_FAILURE } result_t;

// written by callbacks on the runtime's threads, read by the test thread
typedef struct state_t {
#ifdef _WIN32
    CRITICAL_SECTION lock;
#else
    pthread_mutex_t lock;
#endif
    bool connected;
    bool binary_received;
    bool binary_value;
    bool analog_received;
    double analog_value;
    result_t read_result;
    result_t command_result;
    bool control_received;
    uint16_t control_index;
    dnp3_op_type_t control_op_type;
    uint8_t control_count;
    uint32_t control_on_time;
    uint32_t control_off_time;
    dnp3_operate_type_t operate_type;
} state_t;

static void lock(state_t *state)
{
#ifdef _WIN32
    EnterCriticalSection(&state->lock);
#else
    pthread_mutex_lock(&state->lock);
#endif
}

static void unlock(state_t *state)
{
#ifdef _WIN32
    LeaveCriticalSection(&state->lock);
#else
    pthread_mutex_unlock(&state->lock);
#endif
}

static uint64_t now_ms(void)
{
#ifdef _WIN32
    return GetTickCount64();
#else
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000 + (uint64_t)ts.tv_nsec / 1000000;
#endif
}

static void sleep_ms(unsigned ms)
{
#ifdef _WIN32
    Sleep(ms);
#else
    struct timespec ts = {.tv_sec = ms / 1000, .tv_nsec = (long)(ms % 1000) * 1000000};
    nanosleep(&ts, NULL);
#endif
}

// waits until the condition holds, failing after WAIT_TIMEOUT_MS
static bool wait_for(state_t *state, bool (*condition)(const state_t *), const char *what)
{
    uint64_t deadline = now_ms() + WAIT_TIMEOUT_MS;
    for (;;) {
        lock(state);
        bool done = condition(state);
        unlock(state);
        if (done) {
            return true;
        }
        if (now_ms() >= deadline) {
            fprintf(stderr, "timed out waiting for %s\n", what);
            return false;
        }
        sleep_ms(10);
    }
}

static bool is_connected(const state_t *state) { return state->connected; }
static bool read_done(const state_t *state) { return state->read_result != RESULT_PENDING; }
static bool command_done(const state_t *state) { return state->command_result != RESULT_PENDING; }

static bool check(bool condition, const char *what)
{
    if (!condition) {
        fprintf(stderr, "check failed: %s\n", what);
    }
    return condition;
}

static bool check_param(dnp3_param_error_t err, const char *what)
{
    if (err != DNP3_PARAM_ERROR_OK) {
        fprintf(stderr, "%s failed: %s\n", what, dnp3_param_error_to_string(err));
        return false;
    }
    return true;
}

// ---- outstation callbacks ----

static void add_points(dnp3_database_t *db, void *ctx)
{
    dnp3_flags_t online = dnp3_flags_init(DNP3_FLAG_ONLINE);
    dnp3_database_add_binary_input(db, 0, DNP3_EVENT_CLASS_NONE, dnp3_binary_input_config_init());
    dnp3_database_add_analog_input(db, 0, DNP3_EVENT_CLASS_NONE, dnp3_analog_input_config_init());
    dnp3_database_update_binary_input(db, dnp3_binary_input_init(0, true, online, dnp3_timestamp_invalid_timestamp()),
                                      dnp3_update_options_detect_event());
    dnp3_database_update_analog_input(db, dnp3_analog_input_init(0, 42.0, online, dnp3_timestamp_invalid_timestamp()),
                                      dnp3_update_options_detect_event());
}

static dnp3_command_status_t operate_g12v1(dnp3_group12_var1_t control, uint16_t index, dnp3_operate_type_t op_type,
                                           dnp3_database_handle_t *database, void *ctx)
{
    state_t *state = (state_t *)ctx;
    lock(state);
    state->control_received = true;
    state->control_index = index;
    state->control_op_type = control.code.op_type;
    state->control_count = control.count;
    state->control_on_time = control.on_time;
    state->control_off_time = control.off_time;
    state->operate_type = op_type;
    unlock(state);
    return DNP3_COMMAND_STATUS_SUCCESS;
}

// ---- master callbacks ----

static void on_client_state(dnp3_client_state_t client_state, void *ctx)
{
    state_t *state = (state_t *)ctx;
    lock(state);
    state->connected = client_state == DNP3_CLIENT_STATE_CONNECTED;
    unlock(state);
}

static void handle_binary_input(dnp3_header_info_t info, dnp3_binary_input_iterator_t *it, void *ctx)
{
    state_t *state = (state_t *)ctx;
    dnp3_binary_input_t *value = NULL;
    while ((value = dnp3_binary_input_iterator_next(it))) {
        if (value->index == 0) {
            lock(state);
            state->binary_received = true;
            state->binary_value = value->value;
            unlock(state);
        }
    }
}

static void handle_analog_input(dnp3_header_info_t info, dnp3_analog_input_iterator_t *it, void *ctx)
{
    state_t *state = (state_t *)ctx;
    dnp3_analog_input_t *value = NULL;
    while ((value = dnp3_analog_input_iterator_next(it))) {
        if (value->index == 0) {
            lock(state);
            state->analog_received = true;
            state->analog_value = value->value;
            unlock(state);
        }
    }
}

static void on_read_complete(dnp3_nothing_t nothing, void *ctx)
{
    state_t *state = (state_t *)ctx;
    lock(state);
    state->read_result = RESULT_SUCCESS;
    unlock(state);
}

static void on_read_failure(dnp3_read_error_t error, void *ctx)
{
    state_t *state = (state_t *)ctx;
    fprintf(stderr, "read failed: %s\n", dnp3_read_error_to_string(error));
    lock(state);
    state->read_result = RESULT_FAILURE;
    unlock(state);
}

static void on_command_complete(dnp3_nothing_t nothing, void *ctx)
{
    state_t *state = (state_t *)ctx;
    lock(state);
    state->command_result = RESULT_SUCCESS;
    unlock(state);
}

static void on_command_failure(dnp3_command_error_t error, void *ctx)
{
    state_t *state = (state_t *)ctx;
    fprintf(stderr, "command failed: %s\n", dnp3_command_error_to_string(error));
    lock(state);
    state->command_result = RESULT_FAILURE;
    unlock(state);
}

// ---- test ----

static bool start_outstation(dnp3_runtime_t *runtime, state_t *state, dnp3_outstation_server_t **server,
                             dnp3_outstation_t **outstation)
{
    if (!check_param(dnp3_outstation_server_create_tcp_server(runtime, DNP3_LINK_ERROR_MODE_CLOSE, ENDPOINT, server),
                     "create the TCP server")) {
        return false;
    }

    dnp3_outstation_application_t application = {0};
    dnp3_outstation_information_t information = {0};
    dnp3_control_handler_t control_handler = {.operate_g12v1 = &operate_g12v1, .ctx = state};
    dnp3_connection_state_listener_t listener = {0};
    dnp3_address_filter_t *filter = dnp3_address_filter_any();
    dnp3_param_error_t err = dnp3_outstation_server_add_outstation(
        *server, dnp3_outstation_config_init(OUTSTATION_ADDRESS, MASTER_ADDRESS, dnp3_event_buffer_config_no_events()),
        application, information, control_handler, listener, filter, outstation);
    dnp3_address_filter_destroy(filter);
    if (!check_param(err, "add the outstation")) {
        return false;
    }

    dnp3_database_transaction_t transaction = {.execute = &add_points};
    dnp3_outstation_transaction(*outstation, transaction);

    return check_param(dnp3_outstation_server_bind(*server), "bind the server");
}

static bool start_master(dnp3_runtime_t *runtime, state_t *state, dnp3_master_channel_t **channel,
                         dnp3_association_id_t *association)
{
    dnp3_client_state_listener_t listener = {.on_change = &on_client_state, .ctx = state};
    dnp3_endpoint_list_t *endpoints = dnp3_endpoint_list_create(ENDPOINT);
    dnp3_param_error_t err =
        dnp3_master_channel_create_tcp(runtime, DNP3_LINK_ERROR_MODE_CLOSE, dnp3_master_channel_config_init(MASTER_ADDRESS),
                                       endpoints, dnp3_connect_strategy_init(), listener, channel);
    dnp3_endpoint_list_destroy(endpoints);
    if (!check_param(err, "create the master channel")) {
        return false;
    }

    // no startup integrity poll or unsolicited setup, so only the test's own requests are made
    dnp3_association_config_t config = dnp3_association_config_init(dnp3_event_classes_none(), dnp3_event_classes_none(),
                                                                    dnp3_classes_none(), dnp3_event_classes_none());
    dnp3_read_handler_t read_handler = {
        .handle_binary_input = &handle_binary_input,
        .handle_analog_input = &handle_analog_input,
        .ctx = state,
    };
    dnp3_association_handler_t association_handler = {0};
    dnp3_association_information_t association_information = {0};
    if (!check_param(dnp3_master_channel_add_association(*channel, OUTSTATION_ADDRESS, config, read_handler,
                                                         association_handler, association_information, association),
                     "add the association")) {
        return false;
    }

    return check_param(dnp3_master_channel_enable(*channel), "enable the master");
}

static bool read_class0(dnp3_master_channel_t *channel, dnp3_association_id_t association, state_t *state)
{
    dnp3_read_task_callback_t callback = {
        .on_complete = &on_read_complete,
        .on_failure = &on_read_failure,
        .ctx = state,
    };
    dnp3_request_t *request = dnp3_request_new_class(true, false, false, false);
    dnp3_param_error_t err = dnp3_master_channel_read(channel, association, request, callback);
    dnp3_request_destroy(request);
    if (!check_param(err, "start the read") || !wait_for(state, &read_done, "the read")) {
        return false;
    }

    lock(state);
    bool ok = check(state->read_result == RESULT_SUCCESS, "the read succeeds") &&
              check(state->binary_received && state->binary_value, "binary input 0 is read as true") &&
              check(state->analog_received && state->analog_value == 42.0, "analog input 0 is read as 42.0");
    unlock(state);
    return ok;
}

static bool direct_operate(dnp3_master_channel_t *channel, dnp3_association_id_t association, state_t *state)
{
    dnp3_command_task_callback_t callback = {
        .on_complete = &on_command_complete,
        .on_failure = &on_command_failure,
        .ctx = state,
    };
    dnp3_command_set_t *commands = dnp3_command_set_create();
    dnp3_command_set_add_g12_v1_u16(
        commands, 0, dnp3_group12_var1_init(dnp3_control_code_init(DNP3_TRIP_CLOSE_CODE_NUL, false, DNP3_OP_TYPE_LATCH_ON), 1, 1000, 2000));
    dnp3_param_error_t err = dnp3_master_channel_operate(channel, association, DNP3_COMMAND_MODE_DIRECT_OPERATE, commands, callback);
    dnp3_command_set_destroy(commands);
    if (!check_param(err, "start the operate") || !wait_for(state, &command_done, "the operate")) {
        return false;
    }

    lock(state);
    bool ok = check(state->command_result == RESULT_SUCCESS, "the operate succeeds") &&
              check(state->control_received, "the outstation receives the control") &&
              check(state->control_index == 0, "the control is for index 0") &&
              check(state->control_op_type == DNP3_OP_TYPE_LATCH_ON, "the control is latch on") &&
              check(state->control_count == 1 && state->control_on_time == 1000 && state->control_off_time == 2000,
                    "the control has count 1, on time 1000 and off time 2000") &&
              check(state->operate_type == DNP3_OPERATE_TYPE_DIRECT_OPERATE, "the control is a direct operate");
    unlock(state);
    return ok;
}

int main(void)
{
    state_t state = {.read_result = RESULT_PENDING, .command_result = RESULT_PENDING};
#ifdef _WIN32
    InitializeCriticalSection(&state.lock);
#else
    pthread_mutex_init(&state.lock, NULL);
#endif

    dnp3_runtime_t *runtime = NULL;
    dnp3_runtime_config_t runtime_config = dnp3_runtime_config_init();
    runtime_config.num_core_threads = 2;
    if (!check_param(dnp3_runtime_create(runtime_config, &runtime), "create the runtime")) {
        return 1;
    }

    dnp3_outstation_server_t *server = NULL;
    dnp3_outstation_t *outstation = NULL;
    dnp3_master_channel_t *channel = NULL;
    dnp3_association_id_t association;

    bool ok = start_outstation(runtime, &state, &server, &outstation) && start_master(runtime, &state, &channel, &association) &&
              wait_for(&state, &is_connected, "the master to connect") && read_class0(channel, association, &state) &&
              direct_operate(channel, association, &state);

    // the destroy functions accept NULL
    dnp3_master_channel_destroy(channel);
    dnp3_outstation_destroy(outstation);
    dnp3_outstation_server_destroy(server);
    dnp3_runtime_destroy(runtime);

#ifdef _WIN32
    DeleteCriticalSection(&state.lock);
#else
    pthread_mutex_destroy(&state.lock);
#endif

    printf(ok ? "smoke test passed\n" : "smoke test failed\n");
    return ok ? 0 : 1;
}
