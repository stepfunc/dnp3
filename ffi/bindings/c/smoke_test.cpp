// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

// Smoke test of the C++ bindings: an outstation and a master in one process, talking over TCP
// loopback. It checks that the library loads and that calls, callbacks and async completions
// cross the boundary: a class 0 read returns the outstation's values, and a direct operate
// reaches the outstation's control handler.

#include "dnp3.hpp"

#include <chrono>
#include <condition_variable>
#include <exception>
#include <iostream>
#include <mutex>
#include <string>

namespace {

const char *const endpoint = "127.0.0.1:20102";
const uint16_t outstation_address = 1024;
const uint16_t master_address = 1;
const auto wait_timeout = std::chrono::seconds(10);

enum class Result { pending, success, failure };

// written by callbacks on the runtime's threads, read by the test thread
struct State {
    std::mutex mutex;
    std::condition_variable changed;

    bool connected = false;
    bool binary_received = false;
    bool binary_value = false;
    bool analog_received = false;
    double analog_value = 0.0;
    Result read_result = Result::pending;
    Result command_result = Result::pending;
    bool control_received = false;
    uint16_t control_index = 0;
    dnp3::OpType control_op_type = dnp3::OpType::nul;
    uint8_t control_count = 0;
    uint32_t control_on_time = 0;
    uint32_t control_off_time = 0;
    dnp3::OperateType operate_type = dnp3::OperateType::select_before_operate;

    template <class F> void update(F f)
    {
        {
            std::lock_guard<std::mutex> lock(mutex);
            f();
        }
        changed.notify_all();
    }

    template <class P> void wait_for(P predicate, const std::string &what)
    {
        std::unique_lock<std::mutex> lock(mutex);
        if (!changed.wait_for(lock, wait_timeout, predicate)) {
            throw std::runtime_error("timed out waiting for " + what);
        }
    }
};

void check(bool condition, const std::string &what)
{
    if (!condition) {
        throw std::runtime_error("check failed: " + what);
    }
}

// ---- outstation ----

class OutstationApplication : public dnp3::OutstationApplication {
    uint16_t get_processing_delay_ms() override { return 0; }
    dnp3::WriteTimeResult write_absolute_time(uint64_t time) override { return dnp3::WriteTimeResult::not_supported; }
    dnp3::ApplicationIin get_application_iin() override { return dnp3::ApplicationIin(); }
    dnp3::RestartDelay cold_restart() override { return dnp3::RestartDelay::not_supported(); }
    dnp3::RestartDelay warm_restart() override { return dnp3::RestartDelay::not_supported(); }
};

class OutstationInformation : public dnp3::OutstationInformation {
    void process_request_from_idle(const dnp3::RequestHeader &header) override {}
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
public:
    explicit ControlHandler(State &state) : state(state) {}

private:
    State &state;

    void begin_fragment() override {}
    void end_fragment(dnp3::DatabaseHandle &database) override {}

    dnp3::CommandStatus select_g12v1(const dnp3::Group12Var1 &control, uint16_t index, dnp3::DatabaseHandle &database) override
    {
        return dnp3::CommandStatus::not_supported;
    }

    dnp3::CommandStatus operate_g12v1(const dnp3::Group12Var1 &control, uint16_t index, dnp3::OperateType op_type,
                                      dnp3::DatabaseHandle &database) override
    {
        state.update([&]() {
            state.control_received = true;
            state.control_index = index;
            state.control_op_type = control.code.op_type;
            state.control_count = control.count;
            state.control_on_time = control.on_time;
            state.control_off_time = control.off_time;
            state.operate_type = op_type;
        });
        return dnp3::CommandStatus::success;
    }

    dnp3::CommandStatus select_g41v1(int32_t value, uint16_t index, dnp3::DatabaseHandle &database) override
    {
        return dnp3::CommandStatus::not_supported;
    }
    dnp3::CommandStatus operate_g41v1(int32_t value, uint16_t index, dnp3::OperateType op_type, dnp3::DatabaseHandle &database) override
    {
        return dnp3::CommandStatus::not_supported;
    }
    dnp3::CommandStatus select_g41v2(int16_t value, uint16_t index, dnp3::DatabaseHandle &database) override
    {
        return dnp3::CommandStatus::not_supported;
    }
    dnp3::CommandStatus operate_g41v2(int16_t value, uint16_t index, dnp3::OperateType op_type, dnp3::DatabaseHandle &database) override
    {
        return dnp3::CommandStatus::not_supported;
    }
    dnp3::CommandStatus select_g41v3(float value, uint16_t index, dnp3::DatabaseHandle &database) override
    {
        return dnp3::CommandStatus::not_supported;
    }
    dnp3::CommandStatus operate_g41v3(float value, uint16_t index, dnp3::OperateType op_type, dnp3::DatabaseHandle &database) override
    {
        return dnp3::CommandStatus::not_supported;
    }
    dnp3::CommandStatus select_g41v4(double value, uint16_t index, dnp3::DatabaseHandle &database) override
    {
        return dnp3::CommandStatus::not_supported;
    }
    dnp3::CommandStatus operate_g41v4(double value, uint16_t index, dnp3::OperateType op_type, dnp3::DatabaseHandle &database) override
    {
        return dnp3::CommandStatus::not_supported;
    }
};

// ---- master ----

class ReadHandler : public dnp3::ReadHandler {
public:
    explicit ReadHandler(State &state) : state(state) {}

private:
    State &state;

    void handle_binary_input(const dnp3::HeaderInfo &info, dnp3::BinaryInputIterator &it) override
    {
        while (it.next()) {
            const auto value = it.get();
            if (value.index == 0) {
                state.update([&]() {
                    state.binary_received = true;
                    state.binary_value = value.value;
                });
            }
        }
    }

    void handle_analog_input(const dnp3::HeaderInfo &info, dnp3::AnalogInputIterator &it) override
    {
        while (it.next()) {
            const auto value = it.get();
            if (value.index == 0) {
                state.update([&]() {
                    state.analog_received = true;
                    state.analog_value = value.value;
                });
            }
        }
    }
};

class AssociationHandler : public dnp3::AssociationHandler {
    dnp3::UtcTimestamp get_current_time() override { return dnp3::UtcTimestamp::invalid(); }
};

class AssociationInformation : public dnp3::AssociationInformation {
    void task_start(dnp3::TaskType task_type, dnp3::FunctionCode function_code, uint8_t seq) override {}
    void task_success(dnp3::TaskType task_type, dnp3::FunctionCode function_code, uint8_t seq) override {}
    void task_fail(dnp3::TaskType task_type, dnp3::TaskError error) override {}
    void unsolicited_response(bool is_duplicate, uint8_t seq) override {}
};

class ReadTaskCallback : public dnp3::ReadTaskCallback {
public:
    explicit ReadTaskCallback(State &state) : state(state) {}

private:
    State &state;

    void on_complete(dnp3::Nothing result) override
    {
        state.update([&]() { state.read_result = Result::success; });
    }

    void on_failure(dnp3::ReadError error) override
    {
        std::cerr << "read failed: " << dnp3::to_string(error) << std::endl;
        state.update([&]() { state.read_result = Result::failure; });
    }
};

class CommandTaskCallback : public dnp3::CommandTaskCallback {
public:
    explicit CommandTaskCallback(State &state) : state(state) {}

private:
    State &state;

    void on_complete(dnp3::Nothing result) override
    {
        state.update([&]() { state.command_result = Result::success; });
    }

    void on_failure(dnp3::CommandError error) override
    {
        std::cerr << "command failed: " << dnp3::to_string(error) << std::endl;
        state.update([&]() { state.command_result = Result::failure; });
    }
};

// ---- test ----

void run(State &state)
{
    // declared first so it's destroyed last, after the outstation and master
    dnp3::RuntimeConfig runtime_config;
    runtime_config.num_core_threads = 2;
    dnp3::Runtime runtime(runtime_config);

    auto server = dnp3::OutstationServer::create_tcp_server(runtime, dnp3::LinkErrorMode::close, endpoint);
    auto filter = dnp3::AddressFilter::any();
    auto outstation = server.add_outstation(dnp3::OutstationConfig(outstation_address, master_address, dnp3::EventBufferConfig::no_events()),
                                            std::make_unique<OutstationApplication>(), std::make_unique<OutstationInformation>(),
                                            std::make_unique<ControlHandler>(state),
                                            dnp3::functional::connection_state_listener([](dnp3::ConnectionState) {}), filter);
    auto add_points = dnp3::functional::database_transaction([](dnp3::Database &db) {
        const dnp3::Flags online(dnp3::flag::online);
        db.add_binary_input(0, dnp3::EventClass::none, dnp3::BinaryInputConfig());
        db.add_analog_input(0, dnp3::EventClass::none, dnp3::AnalogInputConfig());
        db.update_binary_input(dnp3::BinaryInput(0, true, online, dnp3::Timestamp::invalid_timestamp()), dnp3::UpdateOptions::detect_event());
        db.update_analog_input(dnp3::AnalogInput(0, 42.0, online, dnp3::Timestamp::invalid_timestamp()), dnp3::UpdateOptions::detect_event());
    });
    outstation.transaction(add_points);
    server.bind();

    dnp3::EndpointList endpoints{std::string(endpoint)};
    auto channel = dnp3::MasterChannel::create_tcp_channel(
        runtime, dnp3::LinkErrorMode::close, dnp3::MasterChannelConfig(master_address), endpoints, dnp3::ConnectStrategy(),
        dnp3::functional::client_state_listener([&state](dnp3::ClientState client_state) {
            state.update([&]() { state.connected = client_state == dnp3::ClientState::connected; });
        }));
    // no startup integrity poll or unsolicited setup, so only the test's own requests are made
    const dnp3::AssociationConfig config(dnp3::EventClasses::none(), dnp3::EventClasses::none(), dnp3::Classes::none(),
                                         dnp3::EventClasses::none());
    auto association = channel.add_association(outstation_address, config, std::make_unique<ReadHandler>(state),
                                               std::make_unique<AssociationHandler>(), std::make_unique<AssociationInformation>());
    channel.enable();
    state.wait_for([&]() { return state.connected; }, "the master to connect");

    // a class 0 read returns the outstation's values
    auto request = dnp3::Request::class_request(true, false, false, false);
    channel.read(association, request, std::make_unique<ReadTaskCallback>(state));
    state.wait_for([&]() { return state.read_result != Result::pending; }, "the read");
    {
        std::lock_guard<std::mutex> lock(state.mutex);
        check(state.read_result == Result::success, "the read succeeds");
        check(state.binary_received && state.binary_value, "binary input 0 is read as true");
        check(state.analog_received && state.analog_value == 42.0, "analog input 0 is read as 42.0");
    }

    // a direct operate reaches the outstation's control handler
    dnp3::CommandSet commands;
    commands.add_g12_v1_u16(0, dnp3::Group12Var1(dnp3::ControlCode(dnp3::TripCloseCode::nul, false, dnp3::OpType::latch_on), 1, 1000, 2000));
    channel.operate(association, dnp3::CommandMode::direct_operate, commands, std::make_unique<CommandTaskCallback>(state));
    state.wait_for([&]() { return state.command_result != Result::pending; }, "the operate");
    {
        std::lock_guard<std::mutex> lock(state.mutex);
        check(state.command_result == Result::success, "the operate succeeds");
        check(state.control_received, "the outstation receives the control");
        check(state.control_index == 0, "the control is for index 0");
        check(state.control_op_type == dnp3::OpType::latch_on, "the control is latch on");
        check(state.control_count == 1 && state.control_on_time == 1000 && state.control_off_time == 2000,
              "the control has count 1, on time 1000 and off time 2000");
        check(state.operate_type == dnp3::OperateType::direct_operate, "the control is a direct operate");
    }
}

} // namespace

int main()
{
    State state;
    try {
        run(state);
    }
    catch (const std::exception &ex) {
        std::cerr << ex.what() << std::endl;
        std::cout << "smoke test failed" << std::endl;
        return 1;
    }
    std::cout << "smoke test passed" << std::endl;
    return 0;
}
