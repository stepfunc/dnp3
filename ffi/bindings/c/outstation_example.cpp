// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

#include "dnp3.hpp"

#include <algorithm>
#include <chrono>
#include <cstdlib>
#include <iomanip>
#include <iostream>
#include <memory>
#include <string>
#include <vector>

using namespace dnp3;
using namespace dnp3::functional;

// DNP3 addresses
constexpr uint16_t OUTSTATION_ADDRESS = 1024;
constexpr uint16_t MASTER_ADDRESS = 1;

// default endpoints and serial port, the optional command line argument replaces them
constexpr const char *TCP_ENDPOINT = "127.0.0.1:20000";
constexpr const char *TLS_ENDPOINT = "127.0.0.1:20001";
constexpr const char *UDP_LOCAL_ENDPOINT = "127.0.0.1:20000";
constexpr const char *UDP_REMOTE_ENDPOINT = "127.0.0.1:20001";
constexpr const char *SERIAL_PORT = "/dev/ttyS0";

// TLS files, relative to the repository root
constexpr const char *TLS_DOMAIN = "test.com";
constexpr const char *CA_CHAIN_CA_CERT = "./certs/ca_chain/ca_cert.pem";
constexpr const char *CA_CHAIN_ENTITY_CERT = "./certs/ca_chain/entity2_cert.pem";
constexpr const char *CA_CHAIN_ENTITY_KEY = "./certs/ca_chain/entity2_key.pem";
constexpr const char *SELF_SIGNED_PEER_CERT = "./certs/self_signed/entity1_cert.pem";
constexpr const char *SELF_SIGNED_ENTITY_CERT = "./certs/self_signed/entity2_cert.pem";
constexpr const char *SELF_SIGNED_ENTITY_KEY = "./certs/self_signed/entity2_key.pem";

// number of points of each type in the example's database
constexpr uint16_t POINT_COUNT = 10;

void run_tcp_server(Runtime &runtime, const std::string &endpoint);
void run_tcp_client(Runtime &runtime, const std::string &endpoint);
void run_tcp_client_with_connection_handler(Runtime &runtime);
void run_udp(Runtime &runtime, const std::string &remote_endpoint);
void run_serial(Runtime &runtime, const std::string &port);
void run_tls_server(Runtime &runtime, const std::string &endpoint, const TlsServerConfig &config);
void run_server(OutstationServer &server);
void init_database(Outstation &outstation);
void run_outstation(Outstation &outstation);
void print_help();
OutstationConfig get_outstation_config();
EventBufferConfig get_event_buffer_config();
TlsServerConfig get_tls_ca_config();
TlsServerConfig get_tls_self_signed_config();
std::unique_ptr<OutstationApplication> create_outstation_application();
std::unique_ptr<ControlHandler> create_control_handler();
std::unique_ptr<OutstationInformation> create_outstation_information();
std::unique_ptr<ClientConnectionHandler> create_connection_handler();
Timestamp now();
Flags online();
void print_usage();

int main(int argc, char *argv[])
{
    if (argc < 2 || argc > 3) {
        print_usage();
        return EXIT_FAILURE;
    }
    const std::string transport = argv[1];
    // replaces the endpoint, the UDP remote endpoint or the serial port
    const char *argument = (argc == 3) ? argv[2] : nullptr;

    Logging::configure(LoggingConfig(), logger([](LogLevel level, std::string message) { std::cout << message; }));

    // dropping the runtime shuts down everything running on it
    auto runtime = Runtime(RuntimeConfig());

    try {
        if (transport == "tcp-server") {
            run_tcp_server(runtime, argument ? argument : TCP_ENDPOINT);
        }
        else if (transport == "tcp-client") {
            run_tcp_client(runtime, argument ? argument : TCP_ENDPOINT);
        }
        else if (transport == "tcp-client-manager") {
            run_tcp_client_with_connection_handler(runtime);
        }
        else if (transport == "udp") {
            run_udp(runtime, argument ? argument : UDP_REMOTE_ENDPOINT);
        }
        else if (transport == "serial") {
            run_serial(runtime, argument ? argument : SERIAL_PORT);
        }
        else if (transport == "tls-ca") {
            run_tls_server(runtime, argument ? argument : TLS_ENDPOINT, get_tls_ca_config());
        }
        else if (transport == "tls-self-signed") {
            run_tls_server(runtime, argument ? argument : TLS_ENDPOINT, get_tls_self_signed_config());
        }
        else {
            print_usage();
            return EXIT_FAILURE;
        }
    }
    catch (const std::exception &ex) {
        std::cout << "error: " << ex.what() << std::endl;
        return EXIT_FAILURE;
    }

    return EXIT_SUCCESS;
}

void run_tcp_server(Runtime &runtime, const std::string &endpoint)
{
    std::cout << "Starting TCP server on " << endpoint << std::endl;
    // ANCHOR: create_tcp_server
    auto server = OutstationServer::create_tcp_server(runtime, LinkErrorMode::close, endpoint);
    // ANCHOR_END: create_tcp_server

    run_server(server);
}

void run_tcp_client(Runtime &runtime, const std::string &endpoint)
{
    std::cout << "Starting TCP client to " << endpoint << std::endl;
    EndpointList endpoints(endpoint);
    ConnectOptions options;

    auto outstation = Outstation::create_tcp_client(
        runtime,
        LinkErrorMode::close,
        endpoints,
        ConnectStrategy(),
        options,
        get_outstation_config(),
        create_outstation_application(),
        create_outstation_information(),
        create_control_handler(),
        client_state_listener([](ClientState state) { std::cout << "client state: " << to_string(state) << std::endl; })
    );

    init_database(outstation);
    // a TCP client outstation starts disabled: enable it to connect
    outstation.enable();
    run_outstation(outstation);
}

// a TCP client whose connection handler chooses the endpoints and the delays between attempts
void run_tcp_client_with_connection_handler(Runtime &runtime)
{
    auto outstation = Outstation::create_tcp_client_with_handler(
        runtime,
        LinkErrorMode::close,
        "custom-outstation", // span name for logging
        get_outstation_config(),
        create_outstation_application(),
        create_outstation_information(),
        create_control_handler(),
        client_state_listener([](ClientState state) { std::cout << "client state: " << to_string(state) << std::endl; }),
        create_connection_handler()
    );

    std::cout << "Outstation started with a connection handler: it tries several endpoints with exponential backoff" << std::endl;
    init_database(outstation);
    // a TCP client outstation starts disabled: enable it to connect
    outstation.enable();
    run_outstation(outstation);
}

void run_udp(Runtime &runtime, const std::string &remote_endpoint)
{
    std::cout << "Starting UDP transport from " << UDP_LOCAL_ENDPOINT << " to " << remote_endpoint << std::endl;
    // ANCHOR: create_udp
    OutstationUdpConfig udp_config(UDP_LOCAL_ENDPOINT, remote_endpoint);

    auto outstation = Outstation::create_udp(
        runtime,
        udp_config,
        get_outstation_config(),
        create_outstation_application(),
        create_outstation_information(),
        create_control_handler()
    );
    // ANCHOR_END: create_udp

    init_database(outstation);
    run_outstation(outstation);
}

void run_serial(Runtime &runtime, const std::string &port)
{
    std::cout << "Starting serial transport on " << port << std::endl;
    // ANCHOR: create_serial_server
    auto outstation = Outstation::create_serial_session_2(
        runtime,
        port,
        SerialSettings(),        // 9600 8N1, no flow control
        std::chrono::seconds(1), // retry opening the port every second
        get_outstation_config(),
        create_outstation_application(),
        create_outstation_information(),
        create_control_handler(),
        port_state_listener([](PortState state) { std::cout << "port state: " << to_string(state) << std::endl; })
    );
    // ANCHOR_END: create_serial_server

    init_database(outstation);
    run_outstation(outstation);
}

void run_tls_server(Runtime &runtime, const std::string &endpoint, const TlsServerConfig &config)
{
    std::cout << "Starting TLS server on " << endpoint << std::endl;
    // ANCHOR: create_tls_server
    auto server = OutstationServer::create_tls_server(runtime, LinkErrorMode::close, endpoint, config);
    // ANCHOR_END: create_tls_server

    run_server(server);
}

void run_server(OutstationServer &server)
{
    // ANCHOR: tcp_server_add_outstation
    auto filter = AddressFilter::any();
    auto outstation = server.add_outstation(
        get_outstation_config(),
        create_outstation_application(),
        create_outstation_information(),
        create_control_handler(),
        connection_state_listener([](ConnectionState state) { std::cout << "connection state: " << to_string(state) << std::endl; }),
        filter
    );
    // ANCHOR_END: tcp_server_add_outstation

    // set up the outstation's database before the server starts accepting connections
    init_database(outstation);

    // ANCHOR: tcp_server_bind
    server.bind();
    // ANCHOR_END: tcp_server_bind

    run_outstation(outstation);
}

// add the example's points and attributes to the database
void init_database(Outstation &outstation)
{
    // ANCHOR: database_init_transaction
    auto setup = database_transaction([](Database &db) {
        // initialize POINT_COUNT points of each type
        for (uint16_t i = 0; i < POINT_COUNT; ++i) {
            // you can explicitly specify the configuration for each point ...
            db.add_binary_input(
                i,
                EventClass::class1,
                BinaryInputConfig(StaticBinaryInputVariation::group1_var1, EventBinaryInputVariation::group2_var2)
            );
            // ... or just use the defaults
            db.add_double_bit_binary_input(i, EventClass::class1, DoubleBitBinaryInputConfig());
            db.add_binary_output_status(i, EventClass::class1, BinaryOutputStatusConfig());
            db.add_counter(i, EventClass::class1, CounterConfig());
            db.add_frozen_counter(i, EventClass::class1, FrozenCounterConfig());
            db.add_analog_input(
                i,
                EventClass::class1,
                AnalogInputConfig(StaticAnalogInputVariation::group30_var1, EventAnalogInputVariation::group32_var1, 0.0)
            );
            db.add_analog_output_status(i, EventClass::class1, AnalogOutputStatusConfig());
            db.add_octet_string(i, EventClass::class1);
        }

        // define device attributes made available to the master
        db.define_string_attr(0, false, attribute_variations::device_manufacturers_name, "Step Function I/O");
        db.define_string_attr(0, true, attribute_variations::user_assigned_location, "Bend, OR");
    });
    outstation.transaction(setup);
    // ANCHOR_END: database_init_transaction
}

// run the same console loop regardless of the transport type
void run_outstation(Outstation &outstation)
{
    bool binary_input = false;
    bool double_bit_binary_input = false;
    bool binary_output_status = false;
    double analog_input = 0.0;
    double analog_output_status = 0.0;

    print_help();

    // exit on "x" or at the end of input
    std::string cmd;
    while (std::getline(std::cin, cmd)) {
        if (!cmd.empty() && cmd.back() == '\r') {
            cmd.pop_back();
        }

        if (cmd == "x") {
            return;
        }
        else if (cmd == "help") {
            print_help();
        }
        else if (cmd == "enable") {
            outstation.enable();
        }
        else if (cmd == "disable") {
            outstation.disable();
        }
        else if (cmd == "bi") {
            auto modify = database_transaction([&](Database &db) {
                binary_input = !binary_input;
                db.update_binary_input(BinaryInput(7, binary_input, online(), now()), UpdateOptions::detect_event());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "dbbi") {
            auto modify = database_transaction([&](Database &db) {
                double_bit_binary_input = !double_bit_binary_input;
                auto value = double_bit_binary_input ? DoubleBit::determined_on : DoubleBit::determined_off;
                db.update_double_bit_binary_input(DoubleBitBinaryInput(7, value, online(), now()), UpdateOptions::detect_event());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "bos") {
            auto modify = database_transaction([&](Database &db) {
                binary_output_status = !binary_output_status;
                db.update_binary_output_status(BinaryOutputStatus(7, binary_output_status, online(), now()), UpdateOptions::detect_event());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "co") {
            auto modify = database_transaction([](Database &db) {
                // read the counter, which a freeze may have cleared
                uint32_t value = 0;
                try {
                    value = db.get_counter(7).value;
                }
                catch (const ParamException &) {
                    // the point doesn't exist yet: start from 0
                }
                db.update_counter(Counter(7, value + 1, online(), now()), UpdateOptions::detect_event());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "fco") {
            auto modify = database_transaction([](Database &db) {
                // read the frozen counter, which a freeze may have changed
                uint32_t value = 0;
                try {
                    value = db.get_frozen_counter(7).value;
                }
                catch (const ParamException &) {
                    // the point doesn't exist yet: start from 0
                }
                db.update_frozen_counter(FrozenCounter(7, value + 1, online(), now()), UpdateOptions::detect_event());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "ai") {
            auto modify = database_transaction([&](Database &db) {
                analog_input += 1.0;
                db.update_analog_input(AnalogInput(7, analog_input, online(), now()), UpdateOptions::detect_event());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "aif") {
            auto modify = database_transaction([](Database &db) {
                db.update_flags(7, UpdateFlagsType::analog_input, Flags(flag::comm_lost), now(), UpdateOptions::detect_event());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "aos") {
            auto modify = database_transaction([&](Database &db) {
                analog_output_status += 1.0;
                db.update_analog_output_status(AnalogOutputStatus(7, analog_output_status, online(), now()), UpdateOptions::detect_event());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "os") {
            const std::string text = "Hello";
            const std::vector<uint8_t> value(text.begin(), text.end());
            auto modify = database_transaction([&](Database &db) { db.update_octet_string(7, value, UpdateOptions::detect_event()); });
            outstation.transaction(modify);
        }
        else {
            std::cout << "unknown command: " << cmd << std::endl;
        }
    }
}

void print_help()
{
    std::cout << "Commands (each updates the point at index 7):" << std::endl;
    std::cout << "  help - print this list" << std::endl;
    std::cout << "  x - exit" << std::endl;
    std::cout << "  enable - enable the outstation" << std::endl;
    std::cout << "  disable - disable the outstation" << std::endl;
    std::cout << "  bi - toggle binary input" << std::endl;
    std::cout << "  dbbi - toggle double bit binary input" << std::endl;
    std::cout << "  bos - toggle binary output status" << std::endl;
    std::cout << "  co - increment counter" << std::endl;
    std::cout << "  fco - increment frozen counter" << std::endl;
    std::cout << "  ai - increment analog input" << std::endl;
    std::cout << "  aif - set analog input flag to COMM_LOST" << std::endl;
    std::cout << "  aos - increment analog output status" << std::endl;
    std::cout << "  os - set octet string to 'Hello'" << std::endl;
}

OutstationConfig get_outstation_config()
{
    // ANCHOR: outstation_config
    OutstationConfig config(OUTSTATION_ADDRESS, MASTER_ADDRESS, get_event_buffer_config());
    // override the default application decoding level
    config.decode_level.application = AppDecodeLevel::object_values;
    // ANCHOR_END: outstation_config
    return config;
}

// ANCHOR: event_buffer_config
EventBufferConfig get_event_buffer_config()
{
    return EventBufferConfig(
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

TlsServerConfig get_tls_ca_config()
{
    // ANCHOR: tls_ca_chain_config
    // defaults to CA mode
    TlsServerConfig config(
        TLS_DOMAIN,
        CA_CHAIN_CA_CERT,
        CA_CHAIN_ENTITY_CERT,
        CA_CHAIN_ENTITY_KEY,
        "" // no password
    );
    // ANCHOR_END: tls_ca_chain_config
    return config;
}

TlsServerConfig get_tls_self_signed_config()
{
    // ANCHOR: tls_self_signed_config
    TlsServerConfig config(
        TLS_DOMAIN,
        SELF_SIGNED_PEER_CERT,
        SELF_SIGNED_ENTITY_CERT,
        SELF_SIGNED_ENTITY_KEY,
        "" // no password
    );
    config.certificate_mode = CertificateMode::self_signed;
    // ANCHOR_END: tls_self_signed_config
    return config;
}

// the methods that aren't overridden keep their default behavior
class ExampleOutstationApplication : public OutstationApplication {
public:
    uint16_t get_processing_delay_ms() override { return 0; }

    // ANCHOR: time_sync
    ApplicationIin get_application_iin() override
    {
        ApplicationIin iin;
        iin.need_time = need_time;
        return iin;
    }

    WriteTimeResult write_absolute_time(uint64_t time) override
    {
        // a real device sets its clock here, and sets need_time again when its clock drifts
        std::cout << "write absolute time: " << time << std::endl;
        need_time = false;
        return WriteTimeResult::ok;
    }
    // ANCHOR_END: time_sync

    RestartDelay cold_restart() override { return RestartDelay::not_supported(); }

    RestartDelay warm_restart() override { return RestartDelay::not_supported(); }

    // ANCHOR: freeze
    FreezeResult freeze_counters_all(FreezeType freeze_type, DatabaseHandle &database) override
    {
        return freeze_counters(0, POINT_COUNT - 1, freeze_type, database);
    }

    FreezeResult freeze_counters_range(uint16_t start, uint16_t stop, FreezeType freeze_type, DatabaseHandle &database) override
    {
        return freeze_counters(start, stop, freeze_type, database);
    }

private:
    // freezing at a time and interval isn't supported: those methods keep their default
    FreezeResult freeze_counters(uint16_t start, uint16_t stop, FreezeType freeze_type, DatabaseHandle &database)
    {
        const uint16_t last = std::min<uint16_t>(stop, POINT_COUNT - 1);
        const bool clear = freeze_type == FreezeType::freeze_and_clear;
        std::cout << "freeze counters " << start << " to " << last << " (clear: " << (clear ? "true" : "false") << ")" << std::endl;
        auto transaction = database_transaction([=](Database &db) {
            for (uint32_t index = start; index <= last; ++index) {
                try {
                    const auto counter = db.get_counter(static_cast<uint16_t>(index));
                    const auto time = now();
                    db.update_frozen_counter(FrozenCounter(counter.index, counter.value, counter.flags, time), UpdateOptions::detect_event());
                    if (clear) {
                        db.update_counter(Counter(counter.index, 0, counter.flags, time), UpdateOptions::detect_event());
                    }
                }
                catch (const ParamException &) {
                    // no counter at this index
                }
            }
        });
        database.transaction(transaction);
        return FreezeResult::ok;
    }
    // ANCHOR_END: freeze

public:
    // ANCHOR: save_changes
    bool support_write_analog_dead_bands() override { return true; }

    void begin_write_analog_dead_bands() override { std::cout << "begin writing analog dead-bands" << std::endl; }

    void write_analog_dead_band(uint16_t index, double dead_band) override
    {
        std::cout << "change analog dead-band " << index << " to " << std::setprecision(15) << dead_band << std::endl;
    }

    void end_write_analog_dead_bands() override { std::cout << "end writing analog dead-bands: save them here" << std::endl; }

    void begin_assign_class() override { std::cout << "begin assigning classes" << std::endl; }

    void assign_class_all(PointType point_type, EventClass point_class) override
    {
        std::cout << "assign class " << to_string(point_class) << " to all " << to_string(point_type) << " points" << std::endl;
    }

    void assign_class_range(PointType point_type, uint16_t start, uint16_t stop, EventClass point_class) override
    {
        std::cout << "assign class " << to_string(point_class) << " to " << to_string(point_type) << " points " << start << " to " << stop
                  << std::endl;
    }

    void end_assign_class() override { std::cout << "end assigning classes: save them here" << std::endl; }
    // ANCHOR_END: save_changes

    bool write_string_attr(uint8_t set, uint8_t variation, StringAttr attr_type, const char *value) override
    {
        std::cout << "write device attribute: " << to_string(attr_type) << " set=" << static_cast<unsigned>(set)
                  << " variation=" << static_cast<unsigned>(variation) << " value=" << value << std::endl;
        // allow writing any attribute that has been defined as writable
        return true;
    }

private:
    // ask the master for the time as soon as it connects
    bool need_time = true;
};

std::unique_ptr<OutstationApplication> create_outstation_application()
{
    return std::make_unique<ExampleOutstationApplication>();
}

// ANCHOR: control_handler
class ExampleControlHandler : public ControlHandler {
public:
    void begin_fragment() override {}

    void end_fragment(DatabaseHandle &database) override {}

    CommandStatus select_g12v1(const Group12Var1 &control, uint16_t index, DatabaseHandle &database) override
    {
        if (index < POINT_COUNT && (control.code.op_type == OpType::latch_on || control.code.op_type == OpType::latch_off)) {
            return CommandStatus::success;
        }
        else {
            return CommandStatus::not_supported;
        }
    }

    CommandStatus operate_g12v1(const Group12Var1 &control, uint16_t index, OperateType op_type, DatabaseHandle &database) override
    {
        std::cout << "operate " << to_string(control.code.op_type) << " on binary output " << index << " via " << to_string(op_type) << std::endl;
        if (index < POINT_COUNT && (control.code.op_type == OpType::latch_on || control.code.op_type == OpType::latch_off)) {
            auto status = (control.code.op_type == OpType::latch_on);
            auto transaction = database_transaction([=](Database &db) {
                db.update_binary_output_status(BinaryOutputStatus(index, status, online(), now()), UpdateOptions::detect_event());
            });
            database.transaction(transaction);
            return CommandStatus::success;
        }
        else {
            return CommandStatus::not_supported;
        }
    }

    CommandStatus select_g41v1(int32_t value, uint16_t index, DatabaseHandle &database) override { return select_analog_output(index); }

    CommandStatus operate_g41v1(int32_t value, uint16_t index, OperateType op_type, DatabaseHandle &database) override
    {
        return operate_analog_output(value, index, database);
    }

    CommandStatus select_g41v2(int16_t value, uint16_t index, DatabaseHandle &database) override { return select_analog_output(index); }

    CommandStatus operate_g41v2(int16_t value, uint16_t index, OperateType op_type, DatabaseHandle &database) override
    {
        return operate_analog_output(value, index, database);
    }

    CommandStatus select_g41v3(float value, uint16_t index, DatabaseHandle &database) override { return select_analog_output(index); }

    CommandStatus operate_g41v3(float value, uint16_t index, OperateType op_type, DatabaseHandle &database) override
    {
        return operate_analog_output(value, index, database);
    }

    CommandStatus select_g41v4(double value, uint16_t index, DatabaseHandle &database) override { return select_analog_output(index); }

    CommandStatus operate_g41v4(double value, uint16_t index, OperateType op_type, DatabaseHandle &database) override
    {
        return operate_analog_output(value, index, database);
    }

private:
    CommandStatus select_analog_output(uint16_t index) { return index < POINT_COUNT ? CommandStatus::success : CommandStatus::not_supported; }

    CommandStatus operate_analog_output(double value, uint16_t index, DatabaseHandle &database)
    {
        if (index < POINT_COUNT) {
            auto transaction = database_transaction([=](Database &db) {
                db.update_analog_output_status(AnalogOutputStatus(index, value, online(), now()), UpdateOptions::detect_event());
            });
            database.transaction(transaction);
            return CommandStatus::success;
        }
        else {
            return CommandStatus::not_supported;
        }
    }
};
// ANCHOR_END: control_handler

std::unique_ptr<ControlHandler> create_control_handler()
{
    return std::make_unique<ExampleControlHandler>();
}

// every method keeps the default behavior: do nothing
class ExampleOutstationInformation : public OutstationInformation {
    void process_request_from_idle(const RequestHeader &header) override {}
    void broadcast_received(FunctionCode function_code, BroadcastAction action) override {}
    void enter_solicited_confirm_wait(uint8_t ecsn) override {}
    void solicited_confirm_timeout(uint8_t ecsn) override {}
    void solicited_confirm_received(uint8_t ecsn) override {}
    void solicited_confirm_wait_new_request() override {}
    void wrong_solicited_confirm_seq(uint8_t ecsn, uint8_t seq) override {}
    void unexpected_confirm(bool unsolicited, uint8_t seq) override {}
    void enter_unsolicited_confirm_wait(uint8_t ecsn) override {}
    void unsolicited_confirm_timeout(uint8_t ecsn, bool retry) override {}
    void unsolicited_confirmed(uint8_t ecsn) override {}
    void clear_restart_iin() override {}
};

std::unique_ptr<OutstationInformation> create_outstation_information()
{
    return std::make_unique<ExampleOutstationInformation>();
}

// cycles through several endpoints, each with its own master address, backing off after disconnects
class ExampleConnectionHandler : public ClientConnectionHandler {
public:
    void next(NextEndpointAction &action) override
    {
        // after a full cycle through the endpoints, wait before starting the next one
        if (cycle_completed) {
            std::cout << "connection handler: tried every endpoint, waiting 5 seconds before the next round" << std::endl;
            action.sleep_for(std::chrono::seconds(5));
            cycle_completed = false;
            return;
        }

        const auto target = endpoints[current_endpoint];
        std::cout << "connection handler: connecting to " << target.endpoint << " (endpoint " << (current_endpoint + 1) << " of "
                  << endpoints.size() << ")" << std::endl;
        ConnectionInfo info;
        if (info.set_endpoint(target.endpoint) != ParamError::ok) {
            std::cout << "connection handler: invalid endpoint: " << target.endpoint << std::endl;
            action.sleep_for(std::chrono::seconds(5));
            return;
        }
        // give each endpoint its own master address to demonstrate per-connection overrides
        if (info.set_master_address(target.master_address) != ParamError::ok) {
            std::cout << "connection handler: invalid master address: " << target.master_address << std::endl;
            action.sleep_for(std::chrono::seconds(5));
            return;
        }
        info.set_timeout(std::chrono::seconds(10));
        action.connect_to(info);

        // round-robin through the endpoints
        current_endpoint = (current_endpoint + 1) % endpoints.size();
        if (current_endpoint == 0) {
            cycle_completed = true;
        }
    }

    std::chrono::steady_clock::duration disconnected(const char *addr, const char *hostname) override
    {
        connection_attempts++;
        // 1, 2, 4, 8, 16 and then at most 30 seconds
        const auto delay_seconds = std::min(30, 1 << std::min(connection_attempts - 1, 5));
        std::cout << "connection handler: disconnected from " << addr << ", reconnecting in " << delay_seconds << " seconds (attempt "
                  << connection_attempts << ")" << std::endl;
        return std::chrono::seconds(delay_seconds);
    }

    void connecting(const char *addr, const char *hostname) override { std::cout << "connection handler: connecting to " << addr << std::endl; }

    void connected(const char *addr, const char *hostname) override
    {
        connection_attempts = 0;
        std::cout << "connection handler: connected to " << addr << std::endl;
    }

    void connect_failed(const char *addr, const char *hostname) override
    {
        std::cout << "connection handler: connection to " << addr << " failed" << std::endl;
    }

    void resolution_failed(const char *hostname) override
    {
        std::cout << "connection handler: unable to resolve " << hostname << std::endl;
    }

private:
    struct Target {
        std::string endpoint;
        uint16_t master_address;
    };

    const std::vector<Target> endpoints = {{"127.0.0.1:20000", 1}, {"127.0.0.1:20001", 2}, {"127.0.0.1:20002", 3}};
    size_t current_endpoint = 0;
    int connection_attempts = 0;
    bool cycle_completed = false;
};

std::unique_ptr<ClientConnectionHandler> create_connection_handler()
{
    return std::make_unique<ExampleConnectionHandler>();
}

// current time in milliseconds since the UNIX epoch
Timestamp now()
{
    const auto time_since_epoch = std::chrono::system_clock::now().time_since_epoch();
    return Timestamp::synchronized_timestamp(std::chrono::duration_cast<std::chrono::milliseconds>(time_since_epoch).count());
}

Flags online()
{
    return Flags(flag::online);
}

void print_usage()
{
    std::cout << "usage: cpp_outstation_example <tcp-server|tcp-client|tcp-client-manager|udp|serial|tls-ca|tls-self-signed> [endpoint|serial-port]"
              << std::endl;
}
