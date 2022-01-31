#include "dnp3.hpp"

#include <iostream>
#include <iomanip>
#include <string>

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
    FreezeResult freeze_counters_all(FreezeType freeze_type, Database& database) override {        
        return FreezeResult::not_supported;
    }
    FreezeResult freeze_counters_range(uint16_t start, uint16_t stop, FreezeType freeze_type, Database& database) override {    
        return FreezeResult::not_supported;
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

class MyControlHandler : public ControlHandler {
    void begin_fragment() override {}
    void end_fragment() override {}
    CommandStatus select_g12v1(const Group12Var1& control, uint16_t index, Database& database) override {
        return CommandStatus::not_supported;
    }
    CommandStatus operate_g12v1(const Group12Var1& control, uint16_t index, OperateType op_type, Database& database) override {
        return CommandStatus::not_supported;
    }
    CommandStatus select_g41v1(int32_t control, uint16_t index, Database& database) override {
        return CommandStatus::not_supported;
    }
    CommandStatus operate_g41v1(int32_t control, uint16_t index, OperateType op_type, Database& database) override {
        return CommandStatus::not_supported;
    }
    CommandStatus select_g41v2(int16_t value, uint16_t index, Database& database) override {
        return CommandStatus::not_supported;
    }
    CommandStatus operate_g41v2(int16_t value, uint16_t index, OperateType op_type, Database& database) override {
        return CommandStatus::not_supported;
    }
    CommandStatus select_g41v3(float value, uint16_t index, Database& database) override {
        return CommandStatus::not_supported;
    }
    CommandStatus operate_g41v3(float value, uint16_t index, OperateType op_type, Database& database) override {
        return CommandStatus::not_supported;
    }
    CommandStatus select_g41v4(double value, uint16_t index, Database& database) override {
        return CommandStatus::not_supported;
    }
    CommandStatus operate_g41v4(double value, uint16_t index, OperateType op_type, Database& database) override {
        return CommandStatus::not_supported;
    }
};

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

Flags online()
{
    return Flags(flag::online);
}

Timestamp now()
{
    const auto time_since_epoch = std::chrono::system_clock::now().time_since_epoch();
    return Timestamp::synchronized_timestamp(std::chrono::duration_cast<std::chrono::milliseconds>(time_since_epoch).count());
}

dnp3::OutstationConfig get_outstation_config()
{
    dnp3::OutstationConfig config(1024, 1);
    config.decode_level.application = dnp3::AppDecodeLevel::object_values;
    return config;
}

dnp3::EventBufferConfig get_event_buffer_config()
{    
    return EventBufferConfig(10, 10, 10, 10, 10, 10, 10, 10);
}

void run_outstation(dnp3::Outstation &outstation)
{
    // setup the initial state of the outstation
    auto setup = database_transaction([](Database &db) {
        // add 5 of each type
        for (uint16_t i = 0; i < 10; ++i) {
            db.add_binary_input(i, EventClass::class1, BinaryInputConfig());
            db.add_double_bit_binary_input(i, EventClass::class1, DoubleBitBinaryInputConfig());
            db.add_binary_output_status(i, EventClass::class1, BinaryOutputStatusConfig());
            db.add_counter(i, EventClass::class1, CounterConfig());
            db.add_frozen_counter(i, EventClass::class1, FrozenCounterConfig());
            db.add_analog_input(i, EventClass::class1, AnalogInputConfig());
            db.add_analog_output_status(i, EventClass::class1, AnalogOutputStatusConfig());
            db.add_octet_string(i, EventClass::class1);
        }
    });
    outstation.transaction(setup);

    State state;

    while (true) {
        std::string cmd;
        std::getline(std::cin, cmd);

        if (cmd == "x") {
            return;
        }
        else if (cmd == "bi") {
            auto modify = database_transaction([&](Database &db) {
                state.binary = !state.binary;
                db.update_binary_input(BinaryInput(7, state.binary, online(), now()), UpdateOptions());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "dbbi") {
            auto modify = database_transaction([&](Database &db) {
                state.double_bit_binary = !state.double_bit_binary;
                auto value = state.double_bit_binary ? DoubleBit::determined_on : DoubleBit::determined_off;
                db.update_double_bit_binary_input(DoubleBitBinaryInput(3, value, online(), now()), UpdateOptions());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "bos") {
            auto modify = database_transaction([&](Database &db) {
                state.binary_output_status = !state.binary_output_status;
                db.update_binary_output_status(BinaryOutputStatus(7, state.binary_output_status, online(), now()), UpdateOptions());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "co") {
            auto modify = database_transaction([&](Database &db) {
                state.counter += 1;
                db.update_counter(Counter(7, state.counter, online(), now()), UpdateOptions());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "fco") {
            auto modify = database_transaction([&](Database &db) {
                state.frozen_counter += 1;
                db.update_frozen_counter(FrozenCounter(7, state.frozen_counter, online(), now()), UpdateOptions());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "ai") {
            auto modify = database_transaction([&](Database &db) {
                state.analog += 1;
                db.update_analog_input(AnalogInput(7, state.analog, online(), now()), UpdateOptions());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "aos") {
            auto modify = database_transaction([&](Database &db) {
                state.analog_output_status += 1;
                db.update_analog_output_status(AnalogOutputStatus(7, state.analog_output_status, online(), now()), UpdateOptions());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "os") {
            std::vector<uint8_t> values;
            for (auto x : std::string("hello world!")) {
                values.push_back(x);
            }

            auto modify = database_transaction([&](Database &db) { db.update_octet_string(7, values, UpdateOptions()); });
            outstation.transaction(modify);
        }
        else {
            std::cout << "unknown command: " << cmd << std::endl;
        }
    }
}

void run_server(dnp3::TcpServer &server)
{
    auto filter = AddressFilter::any();
    auto outstation = server.add_outstation(
        get_outstation_config(), get_event_buffer_config(), std::make_unique<MyOutstationApplication>(), std::make_unique<MyOutstationInformation>(),
        std::make_unique<MyControlHandler>(),
        connection_state_listener([](ConnectionState state) { std::cout << "ConnectionState: " << to_string(state) << std::endl; }), filter);

    server.bind();

    run_outstation(outstation);
}

void run_tcp_server(dnp3::Runtime &runtime)
{
    dnp3::TcpServer server(runtime, LinkErrorMode::close, "127.0.0.1:20000");
    run_server(server);
}

void run_tls_server(dnp3::Runtime &runtime, const dnp3::TlsServerConfig &config)
{
    dnp3::TcpServer server = dnp3::TcpServer::create_tls_server(runtime, LinkErrorMode::close, "127.0.0.1:20000", config);
    run_server(server);
}

void run_serial(dnp3::Runtime &runtime)
{    
    auto outstation = dnp3::Outstation::create_serial_session(
        runtime,
        "/dev/pts/4",  // change this to a real port
        dnp3::SerialPortSettings(), // default settings
        get_outstation_config(),
        get_event_buffer_config(),
        std::make_unique<MyOutstationApplication>(),
        std::make_unique<MyOutstationInformation>(),
        std::make_unique<MyControlHandler>()
    );

    run_outstation(outstation);
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
