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
                write_hex_byte(std::cout, byte.value);
                first = false;
            }
            std::cout << "]" << std::endl;
        }    
    }
};

class AssociationHandler : public dnp3::AssociationHandler {
    dnp3::UtcTimestamp get_current_time()
    {
        const auto time_since_epoch = std::chrono::system_clock::now().time_since_epoch();
        return dnp3::UtcTimestamp::valid(std::chrono::duration_cast<std::chrono::milliseconds>(time_since_epoch).count());
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

    channel.enable();

    while (true)
    {
        std::string cmd;
        std::getline(std::cin, cmd);

        if(cmd == "x") {
            return 0;
        }
    }
}
