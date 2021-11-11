#include "dnp3.hpp"

#include <iostream>
#include <iomanip>
#include <string>

std::ostream& write_hex_byte(std::ostream& os, uint8_t value)
{
    os << "0x" << std::hex << std::setw(2) << std::setfill('0') << (int)value;
    return os;
}

std::ostream& operator<<(std::ostream& os, const dnp3::Flags& flags)
{
    return write_hex_byte(os, flags.value);
}

class OutstationApplication : public dnp3::OutstationApplication {
    uint16_t get_processing_delay_ms() override {
        return 0;
    }
    dnp3::WriteTimeResult write_absolute_time(uint64_t time) override {
        return dnp3::WriteTimeResult::not_supported;
    }
    dnp3::ApplicationIin get_application_iin() override {
        return dnp3::ApplicationIin();
    }
    dnp3::RestartDelay cold_restart() override {
        return dnp3::RestartDelay::not_supported();
    }
    dnp3::RestartDelay warm_restart() override {
        return dnp3::RestartDelay::not_supported();
    }
    dnp3::FreezeResult freeze_counters_all(dnp3::FreezeType freeze_type, dnp3::Database& database) override {        
        return dnp3::FreezeResult::not_supported;
    }
    dnp3::FreezeResult freeze_counters_range(uint16_t start, uint16_t stop, dnp3::FreezeType freeze_type, dnp3::Database& database) override {    
        return dnp3::FreezeResult::not_supported;
    }
};

class OutstationInformation : public dnp3::OutstationInformation {
    void process_request_from_idle(const dnp3::RequestHeader& header) override {}
    void broadcast_received(dnp3::FunctionCode function_code, dnp3::BroadcastAction action) override {}
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

class ControlHandler : public dnp3::ControlHandler {
    void begin_fragment() override {}
    void end_fragment() override {}
    dnp3::CommandStatus select_g12v1(const dnp3::Group12Var1& control, uint16_t index, dnp3::Database& database) override {
        return dnp3::CommandStatus::not_supported;
    }
    dnp3::CommandStatus operate_g12v1(const dnp3::Group12Var1& control, uint16_t index, dnp3::OperateType op_type, dnp3::Database& database) override {
        return dnp3::CommandStatus::not_supported;
    }
    dnp3::CommandStatus select_g41v1(int32_t control, uint16_t index, dnp3::Database& database) override {
        return dnp3::CommandStatus::not_supported;
    }
    dnp3::CommandStatus operate_g41v1(int32_t control, uint16_t index, dnp3::OperateType op_type, dnp3::Database& database) override {
        return dnp3::CommandStatus::not_supported;
    }
    dnp3::CommandStatus select_g41v2(int16_t value, uint16_t index, dnp3::Database& database) override {
        return dnp3::CommandStatus::not_supported;
    }
    dnp3::CommandStatus operate_g41v2(int16_t value, uint16_t index, dnp3::OperateType op_type, dnp3::Database& database) override {
        return dnp3::CommandStatus::not_supported;
    }
    dnp3::CommandStatus select_g41v3(float value, uint16_t index, dnp3::Database& database) override {
        return dnp3::CommandStatus::not_supported;
    }
    dnp3::CommandStatus operate_g41v3(float value, uint16_t index, dnp3::OperateType op_type, dnp3::Database& database) override {
        return dnp3::CommandStatus::not_supported;
    }
    dnp3::CommandStatus select_g41v4(double value, uint16_t index, dnp3::Database& database) override {
        return dnp3::CommandStatus::not_supported;
    }
    dnp3::CommandStatus operate_g41v4(double value, uint16_t index, dnp3::OperateType op_type, dnp3::Database& database) override {
        return dnp3::CommandStatus::not_supported;
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

dnp3::Flags online()
{
    return dnp3::Flags(dnp3::flag::online);
}

dnp3::Timestamp now()
{
    const auto time_since_epoch = std::chrono::system_clock::now().time_since_epoch();
    return dnp3::Timestamp::synchronized_timestamp(std::chrono::duration_cast<std::chrono::milliseconds>(time_since_epoch).count());
}

int main()
{
    dnp3::Logging::configure(dnp3::LoggingConfig(), dnp3::functional::logger(
        [](dnp3::LogLevel level, std::string message) {
            std::cout << message;
        }
    ));

    auto runtime = dnp3::Runtime(dnp3::RuntimeConfig());
    
    dnp3::TcpServer server(runtime, dnp3::LinkErrorMode::close, "127.0.0.1:20000");

    auto filter = dnp3::AddressFilter::any();
    auto outstation = server.add_outstation(
        dnp3::OutstationConfig(1024, 1),
        dnp3::EventBufferConfig(10, 10, 10, 10, 10, 10, 10, 10),
        std::make_unique<OutstationApplication>(),
        std::make_unique<OutstationInformation>(),
        std::make_unique<ControlHandler>(),
        dnp3::functional::connection_state_listener([](dnp3::ConnectionState state) {
            std::cout << "ConnectionState: " << dnp3::to_string(state) << std::endl;
        }),
        filter
    );

    auto setup = dnp3::functional::database_transaction([](dnp3::Database& db) {
        // add 5 of each type
        for (uint16_t i = 0; i < 10; ++i) {
            db.add_binary(i, dnp3::EventClass::class1, dnp3::BinaryConfig());
            db.add_double_bit_binary(i, dnp3::EventClass::class1, dnp3::DoubleBitBinaryConfig());
            db.add_binary_output_status(i, dnp3::EventClass::class1, dnp3::BinaryOutputStatusConfig());
            db.add_counter(i, dnp3::EventClass::class1, dnp3::CounterConfig());
            db.add_frozen_counter(i, dnp3::EventClass::class1, dnp3::FrozenCounterConfig());
            db.add_analog(i, dnp3::EventClass::class1, dnp3::AnalogConfig());
            db.add_analog_output_status(i, dnp3::EventClass::class1, dnp3::AnalogOutputStatusConfig());
            db.add_octet_string(i, dnp3::EventClass::class1);
        }
    });
    outstation.transaction(setup);

    server.bind();

    State state;

    while (true)
    {
        std::string cmd;
        std::getline(std::cin, cmd);

        if(cmd == "x") {
            return 0;
        }
        else if (cmd == "bi") {
            auto modify = dnp3::functional::database_transaction([&](dnp3::Database& db) {
                state.binary = !state.binary;
                db.update_binary(dnp3::Binary(3, state.binary, online(), now()), dnp3::UpdateOptions());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "bi") {
            auto modify = dnp3::functional::database_transaction([&](dnp3::Database& db) {
                state.binary = !state.binary;
                db.update_binary(dnp3::Binary(3, state.binary, online(), now()), dnp3::UpdateOptions());
            });
            outstation.transaction(modify);
        }
        else if (cmd == "dbbi") {
            auto modify = dnp3::functional::database_transaction([&](dnp3::Database& db) {
                state.double_bit_binary = !state.double_bit_binary;
                auto value = state.double_bit_binary ? dnp3::DoubleBit::determined_on : dnp3::DoubleBit::determined_off;
                db.update_double_bit_binary(dnp3::DoubleBitBinary(3, value, online(), now()), dnp3::UpdateOptions());
            });
            outstation.transaction(modify);
        }
        else {
            std::cout << "unknown command: " << cmd << std::endl;
        }
    }
}
