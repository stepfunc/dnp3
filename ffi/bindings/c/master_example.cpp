#include "dnp3.hpp"

#include <iostream>
#include <iomanip>
#include <string>

class Logger : public dnp3::Logger {
    void on_message(dnp3::LogLevel level, std::string message) override
    {
        std::cout << message;
    }
};

class ClientStateListener : public dnp3::ClientStateListener {
    void on_change(dnp3::ClientState state) override {
        std::cout << "client state change: " << dnp3::to_string(state) << std::endl;
    }
};

std::ostream& write_hex_byte(std::ostream& os, uint8_t value)
{
    os << "0x" << std::hex << std::setw(2) << std::setfill('0') << (int)value;
    return os;
}

std::ostream& operator<<(std::ostream& os, const dnp3::Flags& flags)
{
    return write_hex_byte(os, flags.value);
}

class ReadHandler : public dnp3::ReadHandler {
    void begin_fragment(dnp3::ReadType read_type, const dnp3::ResponseHeader& header) override {}
    void end_fragment(dnp3::ReadType read_type, const dnp3::ResponseHeader& header) override {}
    void handle_binary(const dnp3::HeaderInfo& info, dnp3::BinaryIterator& it) override {
        while (it.next()) {
            const auto value = it.get();
            std::cout << "BinaryInput(" << value.index << "): value: " << value.value << " flags: " << value.flags << " time: " << value.time.value << std::endl;
        }
    }
    void handle_double_bit_binary(const dnp3::HeaderInfo& info, dnp3::DoubleBitBinaryIterator& it) override {
        while (it.next()) {
            const auto value = it.get();
            std::cout << "DoubleBitBinaryInput(" << value.index << "): value: " << dnp3::to_string(value.value) << " flags: " << value.flags << " time: " << value.time.value << std::endl;
        }
    }
    void handle_binary_output_status(const dnp3::HeaderInfo& info, dnp3::BinaryOutputStatusIterator& it) override {
        while (it.next()) {
            const auto value = it.get();
            std::cout << "BinaryOutputStatus(" << value.index << "): value: " << value.value << " flags: " << value.flags << " time: " << value.time.value << std::endl;
        }
    }
    void handle_counter(const dnp3::HeaderInfo& info, dnp3::CounterIterator& it) override {
        while (it.next()) {
            const auto value = it.get();
            std::cout << "Counter(" << value.index << "): value: " << value.value << " flags: " << value.flags << " time: " << value.time.value << std::endl;
        }
    }
    void handle_frozen_counter(const dnp3::HeaderInfo& info, dnp3::FrozenCounterIterator& it) override {
        while (it.next()) {
            const auto value = it.get();
            std::cout << "FrozenCounter(" << value.index << "): value: " << value.value << " flags: " << value.flags << " time: " << value.time.value << std::endl;
        }
    }
    void handle_analog(const dnp3::HeaderInfo& info, dnp3::AnalogIterator& it) override {
        while (it.next()) {
            const auto value = it.get();
            std::cout << "Analog(" << value.index << "): value: " << value.value << " flags: " << value.flags << " time: " << value.time.value << std::endl;
        }
    }
    void handle_analog_output_status(const dnp3::HeaderInfo& info, dnp3::AnalogOutputStatusIterator& it) override {
        while (it.next()) {
            const auto value = it.get();
            std::cout << "AnalogOutputStatus(" << value.index << "): value: " << value.value << " flags: " << value.flags << " time: " << value.time.value << std::endl;
        }
    }
    void handle_octet_string(const dnp3::HeaderInfo& info, dnp3::OctetStringIterator& it) override {
        while (it.next()) {
            auto value = it.get();
            std::cout << "OctetString(" << value.index << "): value: [";
            bool first = false;
            while (value.value.next()) {
                const auto byte = value.value.get();
                if (!first) {
                    std::cout << ",";
                }
                write_hex_byte(std::cout, byte);
                first = false;
            }
            std::cout << "]" << std::endl;
        }
    }
};

class AssociationHandler : public dnp3::AssociationHandler {
    dnp3::UtcTimestamp get_current_time() override
    {
        const auto time_since_epoch = std::chrono::system_clock::now().time_since_epoch();
        return dnp3::UtcTimestamp::valid(std::chrono::duration_cast<std::chrono::milliseconds>(time_since_epoch).count());
    }
};

class CommandTaskCallback : public dnp3::CommandTaskCallback {
    void on_complete(dnp3::Nothing result) override {
        std::cout << "command succeeded!" << std::endl;
    }
    void on_failure(dnp3::CommandError error) override {
        std::cout << "command failed: "<< dnp3::to_string(error) << std::endl;
    }
};

class ReadTaskCallback : public dnp3::ReadTaskCallback {
    virtual void on_complete(dnp3::Nothing result)
    {
        std::cout << "read succeeded!" << std::endl;
    }

    virtual void on_failure(dnp3::ReadError error) override
    {
        std::cout << "read failed: " << dnp3::to_string(error) << std::endl;
    }
};

class TimeSyncTaskCallback : public dnp3::TimeSyncTaskCallback {
    virtual void on_complete(dnp3::Nothing result)
    {
        std::cout << "time sync succeeded!" << std::endl;
    }

    virtual void on_failure(dnp3::TimeSyncError error) override
    {
        std::cout << "time sync failed: " << dnp3::to_string(error) << std::endl;
    }
};

class RestartTaskCallback : public dnp3::RestartTaskCallback {

    void on_complete(std::chrono::steady_clock::duration result) override
    {
        const auto count = std::chrono::duration_cast<std::chrono::milliseconds>(result).count();
        std::cout << "device will restart in " << count << " milliseconds!" << std::endl;
    }

    void on_failure(dnp3::RestartError error) override
    {
        std::cout << "restart failed failed: " << dnp3::to_string(error) << std::endl;
    }
};

class LinkStatusCallback : public dnp3::LinkStatusCallback {
    void on_complete(dnp3::Nothing result) override
    {
        std::cout << "link status succeeded" << std::endl;
    }

    void on_failure(dnp3::LinkStatusError error) override
    {
        std::cout << "link status failed: " << dnp3::to_string(error) << std::endl;
    }
};

int main()
{
    dnp3::Logging::configure(dnp3::LoggingConfig(), std::make_unique<Logger>());

    auto runtime = dnp3::Runtime(dnp3::RuntimeConfig());

    dnp3::EndpointList endpoints(std::string("127.0.0.1:20000"));
    auto channel = dnp3::MasterChannel::create_tcp_channel(
        runtime,
        dnp3::LinkErrorMode::close,
        dnp3::MasterChannelConfig(1),
        endpoints,
        dnp3::ConnectStrategy(),
        std::make_unique<ClientStateListener>()
    );

    auto assoc = channel.add_association(
        1024,
        dnp3::AssociationConfig(
           dnp3::EventClasses::all(),
           dnp3::EventClasses::all(),
           dnp3::Classes::all(),
           dnp3::EventClasses::none()
        ),
        std::make_unique<ReadHandler>(),
        std::make_unique<AssociationHandler>()
    );

    // ANCHOR: add_poll
    auto event_scan = dnp3::Request::class_request(false, true, true, true);
    const auto event_poll = channel.add_poll(assoc, event_scan, std::chrono::seconds(10));
    // ANCHOR_END: add_poll

    channel.enable();

    while (true)
    {
        std::string cmd;
        std::getline(std::cin, cmd);

        if (cmd == "x") {
            return 0;
        }
        else if (cmd == "enable") {
            channel.enable();
        } else if (cmd == "disable") {
            channel.disable();
        }
        else if (cmd == "dln") {
            channel.set_decode_level(dnp3::DecodeLevel());
        }
        else if (cmd == "dlv") {
            dnp3::DecodeLevel level;
            level.application = dnp3::AppDecodeLevel::object_values;
            channel.set_decode_level(level);
        }
        else if (cmd == "rao") {
            dnp3::Request request;
            request.add_all_objects_header(dnp3::Variation::group40_var0);
            channel.read(assoc, request, std::make_unique<ReadTaskCallback>());
        }
        else if (cmd == "rmo") {
            dnp3::Request request;
            request.add_all_objects_header(dnp3::Variation::group1_var0);
            request.add_all_objects_header(dnp3::Variation::group10_var0);
            channel.read(assoc, request, std::make_unique<ReadTaskCallback>());
        }
        else if (cmd == "evt") {
            channel.demand_poll(event_poll);
        }
        else if (cmd == "lts") {
            channel.synchronize_time(assoc, dnp3::TimeSyncMode::lan, std::make_unique<TimeSyncTaskCallback>());
        }
        else if (cmd == "nts") {
            channel.synchronize_time(assoc, dnp3::TimeSyncMode::non_lan, std::make_unique<TimeSyncTaskCallback>());
        }
        else if (cmd == "crt") {
            channel.cold_restart(assoc, std::make_unique<RestartTaskCallback>());
        }
        else if (cmd == "wrt") {
            channel.warm_restart(assoc, std::make_unique<RestartTaskCallback>());
        }
        else if (cmd == "wrt") {
            channel.warm_restart(assoc, std::make_unique<RestartTaskCallback>());
        }
        else if (cmd == "lsr") {
            channel.check_link_status(assoc, std::make_unique<LinkStatusCallback>());
        }
        else if (cmd == "cmd") {
            dnp3::CommandSet commands;
            commands.add_g12_v1_u8(3, dnp3::Group12Var1(dnp3::ControlCode(dnp3::TripCloseCode::nul, false, dnp3::OpType::latch_on), 0, 1000, 1000));            
            channel.operate(
                assoc,
                dnp3::CommandMode::direct_operate,
                commands,
                std::make_unique<CommandTaskCallback>()
            );
        }
        else {
            std::cout << "unknown command: " << cmd << std::endl;
        }
    }
}
