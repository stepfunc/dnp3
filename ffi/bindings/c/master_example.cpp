#include "dnp3.hpp"

#include <iostream>
#include <iomanip>
#include <string>
#include <cstring>

// ANCHOR: logging_callback
class Logger : public dnp3::Logger {
    void on_message(dnp3::LogLevel level, const char *message) override
    {
        std::cout << message;
    }
};
// ANCHOR_END: logging_callback

class ClientStateListener : public dnp3::ClientStateListener {
    void on_change(dnp3::ClientState state) override {
        std::cout << "client state change: " << dnp3::to_string(state) << std::endl;
    }
};

class PortStateListener : public dnp3::PortStateListener {
    void on_change(dnp3::PortState state) override { std::cout << "port state change: " << dnp3::to_string(state) << std::endl; }
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

// ANCHOR: read_handler
class ReadHandler : public dnp3::ReadHandler {
    void begin_fragment(dnp3::ReadType read_type, const dnp3::ResponseHeader& header) override
    {
        std::cout << "Begin fragment (broadcast: " << header.iin.iin1.broadcast << ")" << std::endl;
    }

    void end_fragment(dnp3::ReadType read_type, const dnp3::ResponseHeader& header) override
    {
        std::cout << "End fragment" << std::endl;
    }

    void handle_binary_input(const dnp3::HeaderInfo &info, dnp3::BinaryInputIterator &it) override
    {
        while (it.next()) {
            const auto value = it.get();
            std::cout << "BinaryInput(" << value.index << "): value: " << value.value << " flags: " << value.flags << " time: " << value.time.value << std::endl;
        }
    }
    void handle_double_bit_binary_input(const dnp3::HeaderInfo &info, dnp3::DoubleBitBinaryInputIterator &it) override
    {
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
    void handle_analog_input(const dnp3::HeaderInfo &info, dnp3::AnalogInputIterator &it) override
    {
        while (it.next()) {
            const auto value = it.get();
            std::cout << "AnalogInput(" << value.index << "): value: " << value.value << " flags: " << value.flags << " time: " << value.time.value << std::endl;
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
// ANCHOR_END: read_handler

// ANCHOR: association_handler
class AssociationHandler : public dnp3::AssociationHandler {
    dnp3::UtcTimestamp get_current_time() override
    {
        const auto time_since_epoch = std::chrono::system_clock::now().time_since_epoch();
        return dnp3::UtcTimestamp::valid(std::chrono::duration_cast<std::chrono::milliseconds>(time_since_epoch).count());
    }
};
// ANCHOR_END: association_handler

// ANCHOR: association_information
class AssociationInformation : public dnp3::AssociationInformation {
    void task_start(dnp3::TaskType task_type, dnp3::FunctionCode function_code, uint8_t seq) override
    {

    }

    void task_success(dnp3::TaskType task_type, dnp3::FunctionCode function_code, uint8_t seq) override
    {

    }

    void task_fail(dnp3::TaskType task_type, dnp3::TaskError error) override
    {

    }

    void unsolicited_response(bool is_duplicate, uint8_t seq) override
    {

    }
};
// ANCHOR_END: association_information

// ANCHOR: assoc_control_callback
class CommandTaskCallback : public dnp3::CommandTaskCallback {
    void on_complete(dnp3::Nothing result) override {
        std::cout << "command succeeded!" << std::endl;
    }
    void on_failure(dnp3::CommandError error) override {
        std::cout << "command failed: "<< dnp3::to_string(error) << std::endl;
    }
};
// ANCHOR_END: assoc_control_callback

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
        std::cout << "restart request failed: " << dnp3::to_string(error) << std::endl;
    }
};

class WriteDeadBandsCallback : public dnp3::WriteAnalogDeadBandsCallback { 

    void on_complete(dnp3::Nothing) override
    {
        std::cout << "write dead-bands succeeded" << std::endl;
    }

    void on_failure(dnp3::WriteDeadBandsError error) override { std::cout << "write dead-bands failed: " << dnp3::to_string(error) << std::endl; }
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

dnp3::MasterChannelConfig get_master_channel_config()
{
    // ANCHOR: master_channel_config
    dnp3::MasterChannelConfig config(1);
    config.decode_level.application = dnp3::AppDecodeLevel::object_values;
    return config;
    // ANCHOR_END: master_channel_config
}

dnp3::AssociationConfig get_association_config()
{
    // ANCHOR: association_config
    dnp3::AssociationConfig config(
        dnp3::EventClasses::all(),
        dnp3::EventClasses::all(), 
        dnp3::Classes::all(),
        dnp3::EventClasses::none()
    );
    // ANCHOR_END: association_config

    return config;
}

void run_channel(dnp3::MasterChannel &channel)
{
    // ANCHOR: association_create
    auto assoc = channel.add_association(
        1024,
        get_association_config(),
        std::make_unique<ReadHandler>(),
        std::make_unique<AssociationHandler>(),
        std::make_unique<AssociationInformation>()
    );
    // ANCHOR_END: association_create

    // ANCHOR: add_poll
    auto event_scan = dnp3::Request::class_request(false, true, true, true);
    const auto event_poll = channel.add_poll(assoc, event_scan, std::chrono::seconds(10));
    // ANCHOR_END: add_poll

    channel.enable();

    while (true) {
        std::string cmd;
        std::getline(std::cin, cmd);

        if (cmd == "x") {
            return;
        }
        else if (cmd == "enable") {
            channel.enable();
        }
        else if (cmd == "disable") {
            channel.disable();
        }
        else if (cmd == "dln") {
            channel.set_decode_level(dnp3::DecodeLevel::nothing());
        }
        else if (cmd == "dlv") {
            auto level = dnp3::DecodeLevel::nothing();
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
        else if (cmd == "wad") {
            dnp3::WriteDeadBandRequest request;
            request.add_g34v1_u8(3, 5);
            request.add_g34v3_u16(2, 2.5);
            channel.write_dead_bands(assoc, request, std::make_unique<WriteDeadBandsCallback>());
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
            // ANCHOR: assoc_control
            dnp3::CommandSet commands;
            commands.add_g12_v1_u8(3, dnp3::Group12Var1(dnp3::ControlCode(dnp3::TripCloseCode::nul, false, dnp3::OpType::latch_on), 0, 1000, 1000));
            channel.operate(assoc, dnp3::CommandMode::direct_operate, commands, std::make_unique<CommandTaskCallback>());
            // ANCHOR_END: assoc_control
        }
        else {
            std::cout << "unknown command: " << cmd << std::endl;
        }
    }
}

void run_tcp_client(dnp3::Runtime &runtime)
{
    // ANCHOR: create_master_tcp_channel
    dnp3::EndpointList endpoints(std::string("127.0.0.1:20000"));

    auto channel = dnp3::MasterChannel::create_tcp_channel(
        runtime,
        dnp3::LinkErrorMode::close,
        get_master_channel_config(), endpoints,
        dnp3::ConnectStrategy(),
        std::make_unique<ClientStateListener>()
    );
    // ANCHOR_END: create_master_tcp_channel

    run_channel(channel);
}

void run_serial(dnp3::Runtime &runtime)
{
    // ANCHOR: create_master_serial_channel
    dnp3::EndpointList endpoints(std::string("127.0.0.1:20000"));

    auto channel = dnp3::MasterChannel::create_serial_channel(
        runtime,
        get_master_channel_config(),
        "/dev/pts/4",
        dnp3::SerialSettings(),
        std::chrono::seconds(5),
        std::make_unique<PortStateListener>()
    );
    // ANCHOR_END: create_master_serial_channel

    run_channel(channel);
}

void run_tls_client(dnp3::Runtime &runtime, const dnp3::TlsClientConfig& tls_config)
{
    // ANCHOR: create_master_tls_channel
    dnp3::EndpointList endpoints(std::string("127.0.0.1:20001"));

    auto channel = dnp3::MasterChannel::create_tls_channel(
        runtime,
        dnp3::LinkErrorMode::close,
        get_master_channel_config(),
        endpoints,
        dnp3::ConnectStrategy(),
        std::make_unique<ClientStateListener>(),
        tls_config
    );
    // ANCHOR_END: create_master_tls_channel

    run_channel(channel);
}

dnp3::TlsClientConfig get_ca_tls_config()
{
    // ANCHOR: tls_ca_chain_config
    // defaults to CA mode
    dnp3::TlsClientConfig config(
        "test.com", 
        "./certs/ca_chain/ca_cert.pem",
        "./certs/ca_chain/entity1_cert.pem",
        "./certs/ca_chain/entity1_key.pem",
        "" // no password
    );
    // ANCHOR_END: tls_ca_chain_config

    return config;
}

dnp3::TlsClientConfig get_self_signed_tls_config()
{
    // ANCHOR: tls_self_signed_config
    dnp3::TlsClientConfig config(
        "test.com", 
        "./certs/self_signed/entity2_cert.pem", 
        "./certs/self_signed/entity1_cert.pem",
        "./certs/self_signed/entity1_key.pem",
        "" // no password
    );

    config.certificate_mode = dnp3::CertificateMode::self_signed;
    // ANCHOR_END: tls_self_signed_config

    return config;
}

int main(int argc, char *argv[])
{
    // ANCHOR: logging_init
    dnp3::Logging::configure(dnp3::LoggingConfig(), std::make_unique<Logger>());
    // ANCHOR_END: logging_init

    // ANCHOR: runtime_create
    auto runtime = dnp3::Runtime(dnp3::RuntimeConfig());
    // ANCHOR_END: runtime_create

    if (argc != 2) {
        std::cout << "you must specify a transport type" << std::endl;
        std::cout << "usage: cpp-outstation-example <channel> (tcp, serial, tls-ca, tls-self-signed)" << std::endl;
        return -1;
    }

    const auto type = argv[1];

    if (strcmp(type, "tcp") == 0) {
        run_tcp_client(runtime);
    }
    else if (strcmp(type, "serial") == 0) {
        run_serial(runtime);
    }
    else if (strcmp(type, "tls-ca") == 0) {
        run_tls_client(runtime, get_ca_tls_config());
    }
    else if (strcmp(type, "tls-self-signed") == 0) {
        run_tls_client(runtime, get_self_signed_tls_config());
    }
    else {
        std::cout << "unknown channel type: " << type << std::endl;
        return -1;
    }

    return 0;
}
