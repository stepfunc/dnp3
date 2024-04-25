#include "dnp3.hpp"

#include <iostream>
#include <iomanip>
#include <string>
#include <cstring>

using namespace dnp3;
using namespace dnp3::functional;

std::ostream& write_hex_byte(std::ostream& os, uint8_t value)
{
    os << "0x" << std::hex << std::setw(2) << std::setfill('0') << (int)value;
    return os;
}

std::ostream& operator<<(std::ostream& os, const Flags& flags)
{
    return write_hex_byte(os, flags.value);
}

Flags online()
{
    return Flags(flag::online);
}

Timestamp now()
{
    const auto time_since_epoch = std::chrono::system_clock::now().time_since_epoch();
    return Timestamp::synchronized_timestamp(std::chrono::duration_cast<std::chrono::milliseconds>(time_since_epoch).count());
}

class MyOutstationApplication : public OutstationApplication {
    uint16_t get_processing_delay_ms() override {
        return 0;
    }
    WriteTimeResult write_absolute_time(uint64_t time) override {
        return WriteTimeResult::not_supported;
    }
    ApplicationIin get_application_iin() override {
        return ApplicationIin();
    }
    RestartDelay cold_restart() override {
        return RestartDelay::not_supported();
    }
    RestartDelay warm_restart() override {
        return RestartDelay::not_supported();
    }
    FreezeResult freeze_counters_all(FreezeType freeze_type, DatabaseHandle& database) override {
        return FreezeResult::not_supported;
    }
    FreezeResult freeze_counters_range(uint16_t start, uint16_t stop, FreezeType freeze_type, DatabaseHandle &database) override {
        return FreezeResult::not_supported;
    }
    bool write_string_attr(uint8_t set, uint8_t variation, StringAttr attr_type, const char *value) override {
        // Allow writing any string attributes that have been defined as writable
        return true;
    }
};

class MyOutstationInformation : public OutstationInformation {
    void process_request_from_idle(const RequestHeader& header) override {}
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

// ANCHOR: control_handler
class MyControlHandler : public ControlHandler {
    void begin_fragment() override {}
    void end_fragment(DatabaseHandle& database) override {}

    CommandStatus select_g12v1(const Group12Var1& control, uint16_t index, DatabaseHandle& database) override {
        if (index < 10 && (control.code.op_type == OpType::latch_on || control.code.op_type == OpType::latch_off))
        {
            return CommandStatus::success;
        }
        else
        {
            return CommandStatus::not_supported;
        }
    }

    CommandStatus operate_g12v1(const Group12Var1 &control, uint16_t index, OperateType op_type, DatabaseHandle &database) override
    {
        if (index < 10 && (control.code.op_type == OpType::latch_on || control.code.op_type == OpType::latch_off))
        {
            auto status = (control.code.op_type == OpType::latch_on);
            auto transaction = functional::database_transaction([=](Database &db) {
                db.update_binary_output_status(BinaryOutputStatus(index, status, online(), now()), UpdateOptions::detect_event());
            });
            database.transaction(transaction);
            return CommandStatus::success;
        }
        else
        {
            return CommandStatus::not_supported;
        }
    }

    CommandStatus select_g41v1(int32_t value, uint16_t index, DatabaseHandle& database) override
    {
        return select_analog_output(index);
    }

    CommandStatus operate_g41v1(int32_t value, uint16_t index, OperateType op_type, DatabaseHandle& database) override
    {
        return operate_analog_output(value, index, database);
    }

    CommandStatus select_g41v2(int16_t value, uint16_t index, DatabaseHandle& database) override
    {
        return select_analog_output(index);
    }

    CommandStatus operate_g41v2(int16_t value, uint16_t index, OperateType op_type, DatabaseHandle& database) override
    {
        return operate_analog_output(value, index, database);
    }

    CommandStatus select_g41v3(float value, uint16_t index, DatabaseHandle& database) override
    {
        return select_analog_output(index);
    }

    CommandStatus operate_g41v3(float value, uint16_t index, OperateType op_type, DatabaseHandle &database) override
    {
        return operate_analog_output(value, index, database);
    }

    CommandStatus select_g41v4(double value, uint16_t index, DatabaseHandle &database) override
    {
        return select_analog_output(index);
    }

    CommandStatus operate_g41v4(double value, uint16_t index, OperateType op_type, DatabaseHandle &database) override
    {
        return operate_analog_output(value, index, database);
    }

private:
    CommandStatus select_analog_output(uint16_t index)
    {
        return index < 10 ? CommandStatus::success : CommandStatus::not_supported;
    }

    CommandStatus operate_analog_output(double value, uint16_t index, DatabaseHandle& database)
    {
        if (index < 10)
        {
            auto transaction = functional::database_transaction(
                [=](Database &db) { db.update_analog_output_status(AnalogOutputStatus(index, value, online(), now()), UpdateOptions::detect_event());
            });
            database.transaction(transaction);
            return CommandStatus::success;
        }
        else
        {
            return CommandStatus::not_supported;
        }
    }
};
// ANCHOR_END: control_handler

class State {
public:
    State() = default;

    bool binary = false;
    bool double_bit_binary = false;
    bool binary_output_status = false;
    uint32_t counter = 0;
    uint32_t frozen_counter = 0;
    double analog = 0.0;
    double analog_output_status = 0.0;

};

// ANCHOR: event_buffer_config
dnp3::EventBufferConfig get_event_buffer_config()
{
    return EventBufferConfig(10, 10, 10, 10, 10, 10, 10, 10);
}
// ANCHOR_END: event_buffer_config

// ANCHOR: create_outstation_config
dnp3::OutstationConfig get_outstation_config()
{
    dnp3::OutstationConfig config(1024, 1, get_event_buffer_config());
    config.decode_level.application = dnp3::AppDecodeLevel::object_values;
    return config;
}
// ANCHOR_END: create_outstation_config

void run_outstation(dnp3::Outstation &outstation)
{
    State state;

    while (true) {
        std::string cmd;
        std::getline(std::cin, cmd);

        if (cmd == "x") {
            return;
        }
        else if (cmd == "enable") {
            outstation.enable();
        }
        else if (cmd == "disable") {
            outstation.disable();
        }
        else if (cmd == "bi") {
            auto modify = database_transaction([&](Database &db) {
                state.binary = !state.binary;
                db.update_binary_input(BinaryInput(7, state.binary, online(), now()), UpdateOptions::detect_event());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "dbbi") {
            auto modify = database_transaction([&](Database &db) {
                state.double_bit_binary = !state.double_bit_binary;
                auto value = state.double_bit_binary ? DoubleBit::determined_on : DoubleBit::determined_off;
                db.update_double_bit_binary_input(DoubleBitBinaryInput(3, value, online(), now()), UpdateOptions::detect_event());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "bos") {
            auto modify = database_transaction([&](Database &db) {
                state.binary_output_status = !state.binary_output_status;
                db.update_binary_output_status(BinaryOutputStatus(7, state.binary_output_status, online(), now()), UpdateOptions::detect_event());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "co") {
            auto modify = database_transaction([&](Database &db) {
                state.counter += 1;
                db.update_counter(Counter(7, state.counter, online(), now()), UpdateOptions::detect_event());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "fco") {
            auto modify = database_transaction([&](Database &db) {
                state.frozen_counter += 1;
                db.update_frozen_counter(FrozenCounter(7, state.frozen_counter, online(), now()), UpdateOptions::detect_event());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "ai") {
            auto modify = database_transaction([&](Database &db) {
                state.analog += 1;
                db.update_analog_input(AnalogInput(7, state.analog, online(), now()), UpdateOptions::detect_event());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "aos") {
            auto modify = database_transaction([&](Database &db) {
                state.analog_output_status += 1;
                db.update_analog_output_status(AnalogOutputStatus(7, state.analog_output_status, online(), now()), UpdateOptions::detect_event());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "os") {
            std::vector<uint8_t> values;
            for (auto x : std::string("hello world!")) {
                values.push_back(x);
            }

            auto modify = database_transaction([&](Database &db) { db.update_octet_string(7, values, UpdateOptions::detect_event()); });
            outstation.transaction(modify);
        }
        else {
            std::cout << "unknown command: " << cmd << std::endl;
        }
    }
}

void run_server(dnp3::OutstationServer &server)
{
    // ANCHOR: tcp_server_add_outstation
    auto filter = AddressFilter::any();
    auto outstation = server.add_outstation(
        get_outstation_config(), std::make_unique<MyOutstationApplication>(), std::make_unique<MyOutstationInformation>(),
        std::make_unique<MyControlHandler>(),
        connection_state_listener([](ConnectionState state) { std::cout << "ConnectionState: " << to_string(state) << std::endl; }), filter);
    // ANCHOR_END: tcp_server_add_outstation

    // setup the initial state of the outstation
    // ANCHOR: database_init_transaction
    auto setup = database_transaction([](Database &db) {
        // add 10 points of each type
        for (uint16_t i = 0; i < 10; ++i) {
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
            db.add_analog_input(i, EventClass::class1, AnalogInputConfig());
            db.add_analog_output_status(i, EventClass::class1, AnalogOutputStatusConfig());
            db.add_octet_string(i, EventClass::class1);
        }

        // define device attributes made available to the master
        db.define_string_attr(0, false, dnp3::attribute_variations::device_manufacturers_name, "Step Function I/O");
        db.define_string_attr(0, true, dnp3::attribute_variations::user_assigned_location, "Bend, OR");
    });
    outstation.transaction(setup);
    // ANCHOR_END: database_init_transaction

    // ANCHOR: tcp_server_bind
    server.bind();
    // ANCHOR_END: tcp_server_bind

    run_outstation(outstation);
}

void run_tcp_server(dnp3::Runtime &runtime)
{
    // ANCHOR: create_tcp_server
    auto server = dnp3::OutstationServer::create_tcp_server(runtime, LinkErrorMode::close, "127.0.0.1:20000");
    // ANCHOR_END: create_tcp_server

    run_server(server);
}

void run_tcp_client(dnp3::Runtime &runtime)
{
    dnp3::EndpointList endpoints(std::string("127.0.0.1:20000"));
    dnp3::ConnectOptions options;

    auto outstation = Outstation::create_tcp_client(
        runtime, LinkErrorMode::discard,
        endpoints,
        dnp3::ConnectStrategy(),
        options,
        get_outstation_config(),
        std::make_unique<MyOutstationApplication>(),
        std::make_unique<MyOutstationInformation>(),
        std::make_unique<MyControlHandler>(),
        client_state_listener([](ClientState state) { std::cout << "ClientState: " << to_string(state) << std::endl; })
    );

    run_outstation(outstation);
}

void run_serial(dnp3::Runtime &runtime)
{
    // ANCHOR: create_serial_server
    auto outstation = dnp3::Outstation::create_serial_session_2(
        runtime,
        "/dev/pts/4",  // change this to a real port
        dnp3::SerialSettings(), // default settings
        std::chrono::seconds(5), // retry the port every 5 seconds
        get_outstation_config(),
        std::make_unique<MyOutstationApplication>(),
        std::make_unique<MyOutstationInformation>(),
        std::make_unique<MyControlHandler>(),
        port_state_listener([](PortState state) { std::cout << "PortState: " << to_string(state) << std::endl; })
    );
    // ANCHOR_END: create_serial_server

    run_outstation(outstation);
}

void run_udp(dnp3::Runtime &runtime)
{
    // ANCHOR: create_udp
    dnp3::OutstationUdpConfig udp_config(
        "127.0.0.1:20000",
        "127.0.0.1:20001"
    );

    auto outstation = dnp3::Outstation::create_udp(
        runtime,
        udp_config,
        get_outstation_config(),
        std::make_unique<MyOutstationApplication>(),
        std::make_unique<MyOutstationInformation>(),
        std::make_unique<MyControlHandler>()
    );

    // ANCHOR_END: create_udp

    run_outstation(outstation);
}

void run_tls_server(dnp3::Runtime &runtime, const dnp3::TlsServerConfig &config)
{
    // ANCHOR: create_tls_server
    dnp3::OutstationServer server = dnp3::OutstationServer::create_tls_server(runtime, LinkErrorMode::close, "127.0.0.1:20001", config);
    // ANCHOR_END: create_tls_server

    run_server(server);
}

dnp3::TlsServerConfig get_tls_ca_config()
{ 
    // ANCHOR: tls_ca_chain_config
    // defaults to CA mode
    dnp3::TlsServerConfig config(
        "test.com",
        "./certs/ca_chain/ca_cert.pem",
        "./certs/ca_chain/entity2_cert.pem",
        "./certs/ca_chain/entity2_key.pem",
        "" // no password
    );
    // ANCHOR_END: tls_ca_chain_config

    return config;
}

dnp3::TlsServerConfig get_tls_self_signed_config()
{
    // ANCHOR: tls_self_signed_config
    dnp3::TlsServerConfig config(
        "test.com", 
        "./certs/self_signed/entity1_cert.pem",
        "./certs/self_signed/entity2_cert.pem",
        "./certs/self_signed/entity2_key.pem",
        "" // no password
    );
    config.certificate_mode = dnp3::CertificateMode::self_signed;
    // ANCHOR_END: tls_self_signed_config

    return config;
}

int main(int argc, char *argv[])
{
    Logging::configure(LoggingConfig(), logger(
        [](LogLevel level, std::string message) {
            std::cout << message;
        }
    ));

    auto runtime = Runtime(RuntimeConfig());

    if (argc != 2) {
        std::cout << "you must specify a transport type" << std::endl;
        std::cout << "usage: cpp-outstation-example <channel> (tcp, serial, tls-ca, tls-self-signed)" << std::endl;
        return -1;
    }

    const auto type = argv[1];

    if (strcmp(type, "tcp") == 0) {
        run_tcp_server(runtime);
    }
    else if (strcmp(type, "tcp-client") == 0) {
        run_tcp_client(runtime);
    }
    else if (strcmp(type, "udp") == 0) {
        run_udp(runtime);
    }
    else if (strcmp(type, "serial") == 0) {
        run_serial(runtime);
    }
    else if (strcmp(type, "tls-ca") == 0) {
        run_tls_server(runtime, get_tls_ca_config());
    }
    else if (strcmp(type, "tls-self-signed") == 0) {
        run_tls_server(runtime, get_tls_self_signed_config());
    }
    else {
        std::cout << "unknown channel type: " << type << std::endl;
        return -1;
    }    

    return 0;
}
