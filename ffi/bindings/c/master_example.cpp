// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

#include "dnp3.hpp"

#include <chrono>
#include <cstdlib>
#include <iomanip>
#include <iostream>
#include <memory>
#include <string>
#include <vector>

// DNP3 addresses
constexpr uint16_t MASTER_ADDRESS = 1;
constexpr uint16_t OUTSTATION_ADDRESS = 1024;

// default endpoints and serial port, the optional command line argument replaces them
constexpr const char *TCP_ENDPOINT = "127.0.0.1:20000";
constexpr const char *TLS_ENDPOINT = "127.0.0.1:20001";
constexpr const char *UDP_LOCAL_ENDPOINT = "127.0.0.1:20001";
constexpr const char *UDP_REMOTE_ENDPOINT = "127.0.0.1:20000";
constexpr const char *SERIAL_PORT = "/dev/ttyS0";

// TLS files, relative to the repository root
constexpr const char *TLS_DOMAIN = "test.com";
constexpr const char *CA_CHAIN_CA_CERT = "./certs/ca_chain/ca_cert.pem";
constexpr const char *CA_CHAIN_ENTITY_CERT = "./certs/ca_chain/entity1_cert.pem";
constexpr const char *CA_CHAIN_ENTITY_KEY = "./certs/ca_chain/entity1_key.pem";
constexpr const char *SELF_SIGNED_PEER_CERT = "./certs/self_signed/entity2_cert.pem";
constexpr const char *SELF_SIGNED_ENTITY_CERT = "./certs/self_signed/entity1_cert.pem";
constexpr const char *SELF_SIGNED_ENTITY_KEY = "./certs/self_signed/entity1_key.pem";

dnp3::MasterChannel create_tcp_channel(dnp3::Runtime &runtime, const std::string &endpoint);
dnp3::MasterChannel create_udp_channel(dnp3::Runtime &runtime, const std::string &local_endpoint);
dnp3::MasterChannel create_serial_channel(dnp3::Runtime &runtime, const std::string &port);
dnp3::MasterChannel create_tls_channel(dnp3::Runtime &runtime, const std::string &endpoint, const dnp3::TlsClientConfig &config);
dnp3::AssociationId add_association(dnp3::MasterChannel &channel);
dnp3::AssociationId add_udp_association(dnp3::MasterChannel &channel, const std::string &remote_endpoint);
void run_channel(dnp3::MasterChannel &channel, dnp3::AssociationId assoc);
void run_command(const std::string &cmd, dnp3::MasterChannel &channel, dnp3::AssociationId assoc, dnp3::PollId event_poll);
void print_help();
dnp3::MasterChannelConfig get_master_channel_config();
dnp3::AssociationConfig get_association_config();
dnp3::TlsClientConfig get_tls_ca_config();
dnp3::TlsClientConfig get_tls_self_signed_config();
std::unique_ptr<dnp3::Logger> create_logger();
std::unique_ptr<dnp3::ReadHandler> create_read_handler();
std::unique_ptr<dnp3::AssociationHandler> create_association_handler();
std::unique_ptr<dnp3::AssociationInformation> create_association_information();
std::unique_ptr<dnp3::ClientStateListener> create_client_state_listener();
std::unique_ptr<dnp3::PortStateListener> create_port_state_listener();
std::unique_ptr<dnp3::ReadTaskCallback> create_read_callback(const char *cmd);
std::unique_ptr<dnp3::CommandTaskCallback> create_command_callback(const char *cmd);
std::unique_ptr<dnp3::CommandTaskCallback> create_no_ack_command_callback(const char *cmd);
std::unique_ptr<dnp3::TimeSyncTaskCallback> create_time_sync_callback(const char *cmd);
std::unique_ptr<dnp3::EmptyResponseCallback> create_empty_response_callback(const char *cmd);
std::unique_ptr<dnp3::RestartTaskCallback> create_restart_callback(const char *cmd);
std::unique_ptr<dnp3::ReadDirectoryCallback> create_read_directory_callback(const char *cmd);
std::unique_ptr<dnp3::FileInfoCallback> create_file_info_callback(const char *cmd);
std::unique_ptr<dnp3::FileReader> create_file_reader();
void start_write_file(dnp3::MasterChannel &channel, dnp3::AssociationId assoc);
std::unique_ptr<dnp3::LinkStatusCallback> create_link_status_callback(const char *cmd);
void print_file_info(const dnp3::FileInfo &info);
void print_usage();

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
    const std::string transport = argv[1];
    // replaces the endpoint, the UDP remote endpoint or the serial port
    const char *argument = (argc == 3) ? argv[2] : nullptr;

    // ANCHOR: logging_init
    dnp3::Logging::configure(dnp3::LoggingConfig(), create_logger());
    // ANCHOR_END: logging_init

    // declared before the runtime so it's destroyed after it: the runtime shuts down
    // first, which stops every task and callback, including those that use the channel
    std::unique_ptr<dnp3::MasterChannel> channel;

    // ANCHOR: runtime_init
    auto runtime = dnp3::Runtime(dnp3::RuntimeConfig());
    // ANCHOR_END: runtime_init

    try {
        if (transport == "tcp-client") {
            channel = std::make_unique<dnp3::MasterChannel>(create_tcp_channel(runtime, argument ? argument : TCP_ENDPOINT));
            run_channel(*channel, add_association(*channel));
        }
        else if (transport == "udp") {
            channel = std::make_unique<dnp3::MasterChannel>(create_udp_channel(runtime, UDP_LOCAL_ENDPOINT));
            run_channel(*channel, add_udp_association(*channel, argument ? argument : UDP_REMOTE_ENDPOINT));
        }
        else if (transport == "serial") {
            channel = std::make_unique<dnp3::MasterChannel>(create_serial_channel(runtime, argument ? argument : SERIAL_PORT));
            run_channel(*channel, add_association(*channel));
        }
        else if (transport == "tls-ca") {
            channel = std::make_unique<dnp3::MasterChannel>(create_tls_channel(runtime, argument ? argument : TLS_ENDPOINT, get_tls_ca_config()));
            run_channel(*channel, add_association(*channel));
        }
        else if (transport == "tls-self-signed") {
            channel = std::make_unique<dnp3::MasterChannel>(create_tls_channel(runtime, argument ? argument : TLS_ENDPOINT, get_tls_self_signed_config()));
            run_channel(*channel, add_association(*channel));
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

dnp3::MasterChannel create_tcp_channel(dnp3::Runtime &runtime, const std::string &endpoint)
{
    // ANCHOR: create_tcp_channel
    dnp3::EndpointList endpoints(endpoint);

    auto channel = dnp3::MasterChannel::create_tcp_channel(
        runtime,
        dnp3::LinkErrorMode::close,
        get_master_channel_config(),
        endpoints,
        dnp3::ConnectStrategy(),
        create_client_state_listener()
    );
    // ANCHOR_END: create_tcp_channel
    return channel;
}

dnp3::MasterChannel create_udp_channel(dnp3::Runtime &runtime, const std::string &local_endpoint)
{
    // ANCHOR: create_udp_channel
    auto channel = dnp3::MasterChannel::create_udp_channel(
        runtime,
        get_master_channel_config(),
        local_endpoint,
        dnp3::LinkReadMode::datagram,
        std::chrono::seconds(5) // retry binding the socket every 5 seconds
    );
    // ANCHOR_END: create_udp_channel
    return channel;
}

dnp3::MasterChannel create_serial_channel(dnp3::Runtime &runtime, const std::string &port)
{
    // ANCHOR: create_serial_channel
    auto channel = dnp3::MasterChannel::create_serial_channel(
        runtime,
        get_master_channel_config(),
        port,
        dnp3::SerialSettings(),  // 9600 8N1, no flow control
        std::chrono::seconds(1), // retry opening the port every second
        create_port_state_listener()
    );
    // ANCHOR_END: create_serial_channel
    return channel;
}

dnp3::MasterChannel create_tls_channel(dnp3::Runtime &runtime, const std::string &endpoint, const dnp3::TlsClientConfig &config)
{
    // ANCHOR: create_tls_channel
    dnp3::EndpointList endpoints(endpoint);

    auto channel = dnp3::MasterChannel::create_tls_channel(
        runtime,
        dnp3::LinkErrorMode::close,
        get_master_channel_config(),
        endpoints,
        dnp3::ConnectStrategy(),
        create_client_state_listener(),
        config
    );
    // ANCHOR_END: create_tls_channel
    return channel;
}

dnp3::AssociationId add_association(dnp3::MasterChannel &channel)
{
    // ANCHOR: association_create
    auto assoc = channel.add_association(
        OUTSTATION_ADDRESS,
        get_association_config(),
        create_read_handler(),
        create_association_handler(),
        create_association_information()
    );
    // ANCHOR_END: association_create
    return assoc;
}

dnp3::AssociationId add_udp_association(dnp3::MasterChannel &channel, const std::string &remote_endpoint)
{
    // ANCHOR: association_create_udp
    auto assoc = channel.add_udp_association(
        OUTSTATION_ADDRESS,
        remote_endpoint,
        get_association_config(),
        create_read_handler(),
        create_association_handler(),
        create_association_information()
    );
    // ANCHOR_END: association_create_udp
    return assoc;
}

void run_channel(dnp3::MasterChannel &channel, dnp3::AssociationId assoc)
{
    // create an event poll
    // ANCHOR: add_poll
    auto event_scan = dnp3::Request::class_request(false, true, true, true);
    const auto event_poll = channel.add_poll(assoc, event_scan, std::chrono::seconds(5));
    // ANCHOR_END: add_poll

    // enable communications
    channel.enable();

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
        else {
            try {
                run_command(cmd, channel, assoc, event_poll);
            }
            catch (const std::exception &ex) {
                std::cout << cmd << ": " << ex.what() << std::endl;
            }
        }
    }
}

// start one command: the callbacks print the result of the asynchronous ones
void run_command(const std::string &cmd, dnp3::MasterChannel &channel, dnp3::AssociationId assoc, dnp3::PollId event_poll)
{
    if (cmd == "enable") {
        channel.enable();
        std::cout << "enable: success" << std::endl;
    }
    else if (cmd == "disable") {
        channel.disable();
        std::cout << "disable: success" << std::endl;
    }
    else if (cmd == "dln") {
        channel.set_decode_level(dnp3::DecodeLevel::nothing());
        std::cout << "dln: success" << std::endl;
    }
    else if (cmd == "dlv") {
        auto level = dnp3::DecodeLevel::nothing();
        level.application = dnp3::AppDecodeLevel::object_values;
        channel.set_decode_level(level);
        std::cout << "dlv: success" << std::endl;
    }
    else if (cmd == "rao") {
        dnp3::Request request;
        request.add_all_objects_header(dnp3::Variation::group40_var0);
        channel.read(assoc, request, create_read_callback("rao"));
    }
    else if (cmd == "rmo") {
        dnp3::Request request;
        request.add_all_objects_header(dnp3::Variation::group10_var0);
        request.add_all_objects_header(dnp3::Variation::group40_var0);
        channel.read(assoc, request, create_read_callback("rmo"));
    }
    else if (cmd == "rco") {
        dnp3::Request request;
        request.add_all_objects_header(dnp3::Variation::group20_var0);
        request.add_all_objects_header(dnp3::Variation::group21_var0);
        channel.read(assoc, request, create_read_callback("rco"));
    }
    else if (cmd == "cmd") {
        // ANCHOR: assoc_control
        dnp3::CommandSet commands;
        commands.add_g12_v1_u16(3, dnp3::Group12Var1(dnp3::ControlCode(dnp3::TripCloseCode::nul, false, dnp3::OpType::latch_on), 1, 1000, 1000));
        channel.operate(assoc, dnp3::CommandMode::select_before_operate, commands, create_command_callback("cmd"));
        // ANCHOR_END: assoc_control
    }
    else if (cmd == "cna") {
        // ANCHOR: assoc_control_no_ack
        dnp3::CommandSet commands;
        commands.add_g12_v1_u16(3, dnp3::Group12Var1(dnp3::ControlCode(dnp3::TripCloseCode::nul, false, dnp3::OpType::latch_on), 1, 1000, 1000));
        channel.operate_no_ack(assoc, commands, create_no_ack_command_callback("cna"));
        // ANCHOR_END: assoc_control_no_ack
    }
    else if (cmd == "evt") {
        // the poll runs later, and its data goes to the read handler
        channel.demand_poll(event_poll);
        std::cout << "evt: poll demanded" << std::endl;
    }
    else if (cmd == "lts") {
        channel.synchronize_time(assoc, dnp3::TimeSyncMode::lan, create_time_sync_callback("lts"));
    }
    else if (cmd == "nts") {
        channel.synchronize_time(assoc, dnp3::TimeSyncMode::non_lan, create_time_sync_callback("nts"));
    }
    else if (cmd == "wad") {
        // ANCHOR: write_dead_bands
        dnp3::WriteDeadBandRequest request;
        request.add_g34v1_u8(3, 5);
        request.add_g34v3_u16(4, 2.5);
        channel.write_dead_bands(assoc, request, create_empty_response_callback("wad"));
        // ANCHOR_END: write_dead_bands
    }
    else if (cmd == "ifz") {
        // copy every counter to its frozen counter
        dnp3::Request request;
        request.add_all_objects_header(dnp3::Variation::group20_var0);
        channel.send_and_expect_empty_response(assoc, dnp3::FunctionCode::immediate_freeze, request, create_empty_response_callback("ifz"));
    }
    else if (cmd == "fcl") {
        // copy every counter to its frozen counter, then clear the counter
        dnp3::Request request;
        request.add_all_objects_header(dnp3::Variation::group20_var0);
        channel.send_and_expect_empty_response(assoc, dnp3::FunctionCode::freeze_clear, request, create_empty_response_callback("fcl"));
    }
    else if (cmd == "fat") {
        // ANCHOR: freeze_at_time
        dnp3::Request request;
        // freeze all the counters once per day relative to the beginning of the current hour
        request.add_time_and_interval(0, 86400000);
        // apply this schedule to all counters
        request.add_all_objects_header(dnp3::Variation::group20_var0);
        channel.send_and_expect_empty_response(assoc, dnp3::FunctionCode::freeze_at_time, request, create_empty_response_callback("fat"));
        // ANCHOR_END: freeze_at_time
    }
    else if (cmd == "rda") {
        // ANCHOR: read_attributes
        dnp3::Request request;
        request.add_specific_attribute(dnp3::attribute_variations::all_attributes_request, 0);
        channel.read(assoc, request, create_read_callback("rda"));
        // ANCHOR_END: read_attributes
    }
    else if (cmd == "wda") {
        // ANCHOR: write_attribute
        dnp3::Request request;
        request.add_string_attribute(dnp3::attribute_variations::user_assigned_location, 0, "Mt. Olympus");
        channel.send_and_expect_empty_response(assoc, dnp3::FunctionCode::write, request, create_empty_response_callback("wda"));
        // ANCHOR_END: write_attribute
    }
    else if (cmd == "ral") {
        dnp3::Request request;
        request.add_specific_attribute(dnp3::attribute_variations::list_of_variations, 0);
        channel.read(assoc, request, create_read_callback("ral"));
    }
    else if (cmd == "crt") {
        channel.cold_restart(assoc, create_restart_callback("crt"));
    }
    else if (cmd == "wrt") {
        channel.warm_restart(assoc, create_restart_callback("wrt"));
    }
    else if (cmd == "rd") {
        // ANCHOR: read_directory
        channel.read_directory(assoc, ".", dnp3::DirReadConfig::defaults(), create_read_directory_callback("rd"));
        // ANCHOR_END: read_directory
    }
    else if (cmd == "gfi") {
        // ANCHOR: get_file_info
        channel.get_file_info(assoc, ".", create_file_info_callback("gfi"));
        // ANCHOR_END: get_file_info
    }
    else if (cmd == "rf") {
        // ANCHOR: read_file
        // this reads the root "directory" file but doesn't parse it
        channel.read_file(assoc, ".", dnp3::FileReadConfig::defaults(), create_file_reader());
        // ANCHOR_END: read_file
        std::cout << "rf: started (the file reader reports the outcome)" << std::endl;
    }
    else if (cmd == "wf") {
        start_write_file(channel, assoc);
    }
    else if (cmd == "lsr") {
        channel.check_link_status(assoc, create_link_status_callback("lsr"));
    }
    else {
        std::cout << cmd << ": unknown command, type help for a list" << std::endl;
    }
}

void print_help()
{
    std::cout << "Commands:" << std::endl;
    std::cout << "  help - print this list" << std::endl;
    std::cout << "  x - exit" << std::endl;
    std::cout << "  enable / disable - enable or disable the channel" << std::endl;
    std::cout << "  dln / dlv - decode nothing / decode object values" << std::endl;
    std::cout << "  rao - read analog output statuses" << std::endl;
    std::cout << "  rmo - read binary and analog output statuses" << std::endl;
    std::cout << "  rco - read counters and frozen counters" << std::endl;
    std::cout << "  cmd - select and operate binary output 3 (latch on)" << std::endl;
    std::cout << "  cna - operate binary output 3 without an answer" << std::endl;
    std::cout << "  evt - demand the event poll" << std::endl;
    std::cout << "  lts / nts - synchronize the time (LAN / non-LAN)" << std::endl;
    std::cout << "  wad - write analog input deadbands" << std::endl;
    std::cout << "  ifz - freeze all counters" << std::endl;
    std::cout << "  fcl - freeze and clear all counters" << std::endl;
    std::cout << "  fat - freeze all counters once a day" << std::endl;
    std::cout << "  rda - read all device attributes" << std::endl;
    std::cout << "  wda - write the user-assigned location attribute" << std::endl;
    std::cout << "  ral - read the list of attribute variations" << std::endl;
    std::cout << "  crt / wrt - cold / warm restart" << std::endl;
    std::cout << "  rd - read the root directory" << std::endl;
    std::cout << "  gfi - get information about the root directory" << std::endl;
    std::cout << "  rf - read the root directory as a file" << std::endl;
    std::cout << "  wf - write hello_world.txt" << std::endl;
    std::cout << "  lsr - check the link status" << std::endl;
}

dnp3::MasterChannelConfig get_master_channel_config()
{
    // ANCHOR: master_channel_config
    dnp3::MasterChannelConfig config(MASTER_ADDRESS);
    config.decode_level.application = dnp3::AppDecodeLevel::object_values;
    // ANCHOR_END: master_channel_config
    return config;
}

dnp3::AssociationConfig get_association_config()
{
    // ANCHOR: association_config
    dnp3::AssociationConfig config(
        // disable unsolicited first (Class 1/2/3)
        dnp3::EventClasses::all(),
        // after the integrity poll, enable unsolicited (Class 1/2/3)
        dnp3::EventClasses::all(),
        // perform startup integrity poll with Class 1/2/3/0
        dnp3::Classes::all(),
        // don't automatically scan Class 1/2/3 when the corresponding IIN bit is asserted
        dnp3::EventClasses::none()
    );
    config.auto_time_sync = dnp3::AutoTimeSync::lan;
    config.keep_alive_timeout = std::chrono::seconds(60);
    // ANCHOR_END: association_config
    return config;
}

dnp3::TlsClientConfig get_tls_ca_config()
{
    // ANCHOR: tls_ca_chain_config
    // defaults to CA mode
    dnp3::TlsClientConfig config(
        TLS_DOMAIN,
        CA_CHAIN_CA_CERT,
        CA_CHAIN_ENTITY_CERT,
        CA_CHAIN_ENTITY_KEY,
        "" // no password
    );
    // ANCHOR_END: tls_ca_chain_config
    return config;
}

dnp3::TlsClientConfig get_tls_self_signed_config()
{
    // ANCHOR: tls_self_signed_config
    dnp3::TlsClientConfig config(
        TLS_DOMAIN,
        SELF_SIGNED_PEER_CERT,
        SELF_SIGNED_ENTITY_CERT,
        SELF_SIGNED_ENTITY_KEY,
        "" // no password
    );
    config.certificate_mode = dnp3::CertificateMode::self_signed;
    // ANCHOR_END: tls_self_signed_config
    return config;
}

// ANCHOR: logging_interface
class Logger : public dnp3::Logger {
    void on_message(dnp3::LogLevel level, const char *message) override { std::cout << message; }
};

std::unique_ptr<dnp3::Logger> create_logger()
{
    return std::make_unique<Logger>();
}
// ANCHOR_END: logging_interface

// ANCHOR: read_handler
// print helpers, defined after the handler
void print_header(const char *name, const dnp3::HeaderInfo &info);
void print_flags_and_time(const dnp3::Flags &flags, const dnp3::Timestamp &time);
void print_time(const dnp3::Timestamp &time);
void print_bytes(dnp3::ByteIterator &bytes);
void print_attribute_start(const dnp3::HeaderInfo &info, uint8_t set, uint8_t variation);

class ReadHandler : public dnp3::ReadHandler {
    void begin_fragment(dnp3::ReadType read_type, const dnp3::ResponseHeader &header) override
    {
        std::cout << "begin fragment: " << dnp3::to_string(read_type) << " (broadcast: " << (header.iin.iin1.broadcast ? "true" : "false") << ")"
                  << std::endl;
    }

    void end_fragment(dnp3::ReadType read_type, const dnp3::ResponseHeader &header) override { std::cout << "end fragment" << std::endl; }

    void handle_binary_input(const dnp3::HeaderInfo &info, dnp3::BinaryInputIterator &it) override
    {
        print_header("Binary inputs", info);
        while (it.next()) {
            const auto value = it.get();
            std::cout << "BI " << value.index << ": value=" << (value.value ? "true" : "false");
            print_flags_and_time(value.flags, value.time);
        }
    }

    void handle_double_bit_binary_input(const dnp3::HeaderInfo &info, dnp3::DoubleBitBinaryInputIterator &it) override
    {
        print_header("Double-bit binary inputs", info);
        while (it.next()) {
            const auto value = it.get();
            std::cout << "DBBI " << value.index << ": value=" << dnp3::to_string(value.value);
            print_flags_and_time(value.flags, value.time);
        }
    }

    void handle_binary_output_status(const dnp3::HeaderInfo &info, dnp3::BinaryOutputStatusIterator &it) override
    {
        print_header("Binary output statuses", info);
        while (it.next()) {
            const auto value = it.get();
            std::cout << "BOS " << value.index << ": value=" << (value.value ? "true" : "false");
            print_flags_and_time(value.flags, value.time);
        }
    }

    void handle_counter(const dnp3::HeaderInfo &info, dnp3::CounterIterator &it) override
    {
        print_header("Counters", info);
        while (it.next()) {
            const auto value = it.get();
            std::cout << "C " << value.index << ": value=" << value.value;
            print_flags_and_time(value.flags, value.time);
        }
    }

    void handle_frozen_counter(const dnp3::HeaderInfo &info, dnp3::FrozenCounterIterator &it) override
    {
        print_header("Frozen counters", info);
        while (it.next()) {
            const auto value = it.get();
            std::cout << "FC " << value.index << ": value=" << value.value;
            print_flags_and_time(value.flags, value.time);
        }
    }

    void handle_analog_input(const dnp3::HeaderInfo &info, dnp3::AnalogInputIterator &it) override
    {
        print_header("Analog inputs", info);
        while (it.next()) {
            const auto value = it.get();
            std::cout << "AI " << value.index << ": value=" << std::setprecision(15) << value.value;
            print_flags_and_time(value.flags, value.time);
        }
    }

    void handle_frozen_analog_input(const dnp3::HeaderInfo &info, dnp3::FrozenAnalogInputIterator &it) override
    {
        print_header("Frozen analog inputs", info);
        while (it.next()) {
            const auto value = it.get();
            std::cout << "FAI " << value.index << ": value=" << std::setprecision(15) << value.value;
            print_flags_and_time(value.flags, value.time);
        }
    }

    void handle_analog_input_dead_band(const dnp3::HeaderInfo &info, dnp3::AnalogInputDeadBandIterator &it) override
    {
        print_header("Analog input deadbands", info);
        while (it.next()) {
            const auto value = it.get();
            std::cout << "AIDB " << value.index << ": value=" << std::setprecision(15) << value.value << std::endl;
        }
    }

    void handle_analog_output_status(const dnp3::HeaderInfo &info, dnp3::AnalogOutputStatusIterator &it) override
    {
        print_header("Analog output statuses", info);
        while (it.next()) {
            const auto value = it.get();
            std::cout << "AOS " << value.index << ": value=" << std::setprecision(15) << value.value;
            print_flags_and_time(value.flags, value.time);
        }
    }

    void handle_binary_output_command_event(const dnp3::HeaderInfo &info, dnp3::BinaryOutputCommandEventIterator &it) override
    {
        print_header("Binary output command events", info);
        while (it.next()) {
            const auto value = it.get();
            std::cout << "BOCE " << value.index << ": commanded state=" << (value.commanded_state ? "true" : "false")
                      << " status=" << dnp3::to_string(value.status) << " time=";
            print_time(value.time);
            std::cout << std::endl;
        }
    }

    void handle_analog_output_command_event(const dnp3::HeaderInfo &info, dnp3::AnalogOutputCommandEventIterator &it) override
    {
        print_header("Analog output command events", info);
        while (it.next()) {
            const auto value = it.get();
            std::cout << "AOCE " << value.index << ": commanded value=" << std::setprecision(15) << value.commanded_value << " status=" << dnp3::to_string(value.status)
                      << " time=";
            print_time(value.time);
            std::cout << std::endl;
        }
    }

    void handle_unsigned_integer(const dnp3::HeaderInfo &info, dnp3::UnsignedIntegerIterator &it) override
    {
        print_header("Unsigned integers", info);
        while (it.next()) {
            const auto value = it.get();
            std::cout << "UI " << value.index << ": value=" << static_cast<unsigned>(value.value) << std::endl;
        }
    }

    void handle_octet_string(const dnp3::HeaderInfo &info, dnp3::OctetStringIterator &it) override
    {
        print_header("Octet strings", info);
        while (it.next()) {
            auto value = it.get();
            std::cout << "OS " << value.index << ": value=";
            print_bytes(value.value);
            std::cout << std::endl;
        }
    }

    void handle_abs_time(const dnp3::HeaderInfo &info, const dnp3::Timestamp &time) override
    {
        print_header("Absolute time", info);
        std::cout << "time=" << time.value << std::endl;
    }

    void handle_string_attr(const dnp3::HeaderInfo &info, dnp3::StringAttr attr, uint8_t set, uint8_t variation, const char *value) override
    {
        print_attribute_start(info, set, variation);
        std::cout << value << std::endl;
    }

    void handle_variation_list_attr(const dnp3::HeaderInfo &info, dnp3::VariationListAttr attr, uint8_t set, uint8_t variation,
                                    dnp3::AttrItemIter &items) override
    {
        print_attribute_start(info, set, variation);
        std::cout << "[";
        bool first = true;
        while (items.next()) {
            const auto item = items.get();
            std::cout << (first ? "" : ", ") << static_cast<unsigned>(item.variation)
                      << (item.properties.is_writable ? " (writable)" : "");
            first = false;
        }
        std::cout << "]" << std::endl;
    }

    void handle_uint_attr(const dnp3::HeaderInfo &info, dnp3::UintAttr attr, uint8_t set, uint8_t variation, uint32_t value) override
    {
        print_attribute_start(info, set, variation);
        std::cout << value << std::endl;
    }

    void handle_bool_attr(const dnp3::HeaderInfo &info, dnp3::BoolAttr attr, uint8_t set, uint8_t variation, bool value) override
    {
        print_attribute_start(info, set, variation);
        std::cout << (value ? "true" : "false") << std::endl;
    }

    void handle_int_attr(const dnp3::HeaderInfo &info, dnp3::IntAttr attr, uint8_t set, uint8_t variation, int32_t value) override
    {
        print_attribute_start(info, set, variation);
        std::cout << value << std::endl;
    }

    void handle_time_attr(const dnp3::HeaderInfo &info, dnp3::TimeAttr attr, uint8_t set, uint8_t variation, uint64_t value) override
    {
        print_attribute_start(info, set, variation);
        std::cout << value << std::endl;
    }

    void handle_float_attr(const dnp3::HeaderInfo &info, dnp3::FloatAttr attr, uint8_t set, uint8_t variation, double value) override
    {
        print_attribute_start(info, set, variation);
        std::cout << std::setprecision(15) << value << std::endl;
    }

    void handle_octet_string_attr(const dnp3::HeaderInfo &info, dnp3::OctetStringAttr attr, uint8_t set, uint8_t variation,
                                  dnp3::ByteIterator &value) override
    {
        print_attribute_start(info, set, variation);
        print_bytes(value);
        std::cout << std::endl;
    }

    void handle_bit_string_attr(const dnp3::HeaderInfo &info, dnp3::BitStringAttr attr, uint8_t set, uint8_t variation,
                                dnp3::ByteIterator &value) override
    {
        print_attribute_start(info, set, variation);
        print_bytes(value);
        std::cout << std::endl;
    }
};

std::unique_ptr<dnp3::ReadHandler> create_read_handler()
{
    return std::make_unique<ReadHandler>();
}

void print_header(const char *name, const dnp3::HeaderInfo &info)
{
    std::cout << name << " (" << dnp3::to_string(info.qualifier) << ", " << dnp3::to_string(info.variation) << ")" << std::endl;
}

// ends the line of a measurement with its flags and time
void print_flags_and_time(const dnp3::Flags &flags, const dnp3::Timestamp &time)
{
    std::cout << " flags=0x" << std::hex << std::uppercase << std::setw(2) << std::setfill('0') << static_cast<unsigned>(flags.value) << std::dec
              << std::nouppercase << std::setfill(' ') << " time=";
    print_time(time);
    std::cout << std::endl;
}

void print_time(const dnp3::Timestamp &time)
{
    if (time.quality == dnp3::TimeQuality::synchronized_time) {
        std::cout << time.value << " (synchronized)";
    }
    else if (time.quality == dnp3::TimeQuality::unsynchronized_time) {
        std::cout << time.value << " (unsynchronized)";
    }
    else {
        std::cout << "none";
    }
}

// prints the bytes as a list of hex values
void print_bytes(dnp3::ByteIterator &bytes)
{
    std::cout << "[" << std::hex << std::uppercase << std::setfill('0');
    bool first = true;
    while (bytes.next()) {
        std::cout << (first ? "" : ", ") << std::setw(2) << static_cast<unsigned>(bytes.get());
        first = false;
    }
    std::cout << std::dec << std::nouppercase << std::setfill(' ') << "]";
}

// prints the header and the start of an attribute line, up to its value
void print_attribute_start(const dnp3::HeaderInfo &info, uint8_t set, uint8_t variation)
{
    print_header("Device attributes", info);
    std::cout << "attribute set=" << static_cast<unsigned>(set) << " variation=" << static_cast<unsigned>(variation) << " value=";
}
// ANCHOR_END: read_handler

// ANCHOR: association_handler
class AssociationHandler : public dnp3::AssociationHandler {
    dnp3::UtcTimestamp get_current_time() override
    {
        const auto time_since_epoch = std::chrono::system_clock::now().time_since_epoch();
        return dnp3::UtcTimestamp::valid(std::chrono::duration_cast<std::chrono::milliseconds>(time_since_epoch).count());
    }
};

std::unique_ptr<dnp3::AssociationHandler> create_association_handler()
{
    return std::make_unique<AssociationHandler>();
}
// ANCHOR_END: association_handler

// ANCHOR: association_information
// every method keeps the default behavior: do nothing
class AssociationInformation : public dnp3::AssociationInformation {
    void task_start(dnp3::TaskType task_type, dnp3::FunctionCode function_code, uint8_t seq) override {}
    void task_success(dnp3::TaskType task_type, dnp3::FunctionCode function_code, uint8_t seq) override {}
    void task_fail(dnp3::TaskType task_type, dnp3::TaskError error) override {}
    void unsolicited_response(bool is_duplicate, uint8_t seq) override {}
};

std::unique_ptr<dnp3::AssociationInformation> create_association_information()
{
    return std::make_unique<AssociationInformation>();
}
// ANCHOR_END: association_information

// listeners that print each state change of a client connection or serial port
std::unique_ptr<dnp3::ClientStateListener> create_client_state_listener()
{
    return dnp3::functional::client_state_listener([](dnp3::ClientState state) { std::cout << "client state: " << dnp3::to_string(state) << std::endl; });
}

std::unique_ptr<dnp3::PortStateListener> create_port_state_listener()
{
    return dnp3::functional::port_state_listener([](dnp3::PortState state) { std::cout << "port state: " << dnp3::to_string(state) << std::endl; });
}

// prints "<cmd>: success" or "<cmd>: <error>" when a task that returns nothing completes
template <class Callback, class Error> class PrintResult : public Callback {
public:
    explicit PrintResult(const char *cmd) : cmd(cmd) {}

    void on_complete(dnp3::Nothing result) override { std::cout << cmd << ": success" << std::endl; }

    void on_failure(Error error) override { std::cout << cmd << ": " << dnp3::to_string(error) << std::endl; }

private:
    const char *cmd;
};

std::unique_ptr<dnp3::ReadTaskCallback> create_read_callback(const char *cmd)
{
    return std::make_unique<PrintResult<dnp3::ReadTaskCallback, dnp3::ReadError>>(cmd);
}

// ANCHOR: assoc_control_callback
class CommandTaskCallback : public dnp3::CommandTaskCallback {
public:
    explicit CommandTaskCallback(const char *cmd) : cmd(cmd) {}

    void on_complete(dnp3::Nothing result) override { std::cout << cmd << ": success" << std::endl; }

    void on_failure(dnp3::CommandError error) override { std::cout << cmd << ": " << dnp3::to_string(error) << std::endl; }

private:
    const char *cmd;
};

std::unique_ptr<dnp3::CommandTaskCallback> create_command_callback(const char *cmd)
{
    return std::make_unique<CommandTaskCallback>(cmd);
}
// ANCHOR_END: assoc_control_callback

// ANCHOR: assoc_control_no_ack_callback
class NoAckCommandTaskCallback : public dnp3::CommandTaskCallback {
public:
    explicit NoAckCommandTaskCallback(const char *cmd) : cmd(cmd) {}

    // the request was sent: the outstation doesn't answer
    void on_complete(dnp3::Nothing result) override { std::cout << cmd << ": sent" << std::endl; }

    void on_failure(dnp3::CommandError error) override { std::cout << cmd << ": " << dnp3::to_string(error) << std::endl; }

private:
    const char *cmd;
};

std::unique_ptr<dnp3::CommandTaskCallback> create_no_ack_command_callback(const char *cmd)
{
    return std::make_unique<NoAckCommandTaskCallback>(cmd);
}
// ANCHOR_END: assoc_control_no_ack_callback

std::unique_ptr<dnp3::TimeSyncTaskCallback> create_time_sync_callback(const char *cmd)
{
    return std::make_unique<PrintResult<dnp3::TimeSyncTaskCallback, dnp3::TimeSyncError>>(cmd);
}

std::unique_ptr<dnp3::EmptyResponseCallback> create_empty_response_callback(const char *cmd)
{
    return std::make_unique<PrintResult<dnp3::EmptyResponseCallback, dnp3::EmptyResponseError>>(cmd);
}

class RestartTaskCallback : public dnp3::RestartTaskCallback {
public:
    explicit RestartTaskCallback(const char *cmd) : cmd(cmd) {}

    void on_complete(std::chrono::steady_clock::duration delay) override
    {
        std::cout << "restart delay: " << std::chrono::duration_cast<std::chrono::milliseconds>(delay).count() << " ms" << std::endl;
        std::cout << cmd << ": success" << std::endl;
    }

    void on_failure(dnp3::RestartError error) override { std::cout << cmd << ": " << dnp3::to_string(error) << std::endl; }

private:
    const char *cmd;
};

std::unique_ptr<dnp3::RestartTaskCallback> create_restart_callback(const char *cmd)
{
    return std::make_unique<RestartTaskCallback>(cmd);
}

// ANCHOR: read_directory_callback
class ReadDirectoryCallback : public dnp3::ReadDirectoryCallback {
public:
    explicit ReadDirectoryCallback(const char *cmd) : cmd(cmd) {}

    void on_complete(dnp3::FileInfoIterator &iter) override
    {
        while (iter.next()) {
            print_file_info(iter.get());
        }
        std::cout << cmd << ": success" << std::endl;
    }

    void on_failure(dnp3::FileError error) override { std::cout << cmd << ": " << dnp3::to_string(error) << std::endl; }

private:
    const char *cmd;
};

std::unique_ptr<dnp3::ReadDirectoryCallback> create_read_directory_callback(const char *cmd)
{
    return std::make_unique<ReadDirectoryCallback>(cmd);
}
// ANCHOR_END: read_directory_callback

// ANCHOR: file_info_callback
class FileInfoCallback : public dnp3::FileInfoCallback {
public:
    explicit FileInfoCallback(const char *cmd) : cmd(cmd) {}

    void on_complete(const dnp3::FileInfo &info) override
    {
        print_file_info(info);
        std::cout << cmd << ": success" << std::endl;
    }

    void on_failure(dnp3::FileError error) override { std::cout << cmd << ": " << dnp3::to_string(error) << std::endl; }

private:
    const char *cmd;
};

std::unique_ptr<dnp3::FileInfoCallback> create_file_info_callback(const char *cmd)
{
    return std::make_unique<FileInfoCallback>(cmd);
}
// ANCHOR_END: file_info_callback

// ANCHOR: file_logger
class FileReader : public dnp3::FileReader {
    bool opened(uint32_t size) override
    {
        std::cout << "File opened - size: " << size << std::endl;
        return true; // continue
    }

    bool block_received(uint32_t block_num, dnp3::ByteIterator &data) override
    {
        uint32_t size = 0;
        while (data.next()) {
            ++size;
        }
        std::cout << "Received block " << block_num << " with size: " << size << std::endl;
        return true; // continue
    }

    void aborted(dnp3::FileError error) override { std::cout << "rf: aborted: " << dnp3::to_string(error) << std::endl; }

    void completed() override { std::cout << "rf: completed" << std::endl; }
};

std::unique_ptr<dnp3::FileReader> create_file_reader()
{
    return std::make_unique<FileReader>();
}
// ANCHOR_END: file_logger

// ANCHOR: write_file
void write_hello_world_block(dnp3::MasterChannel &channel, dnp3::AssociationId assoc, uint32_t handle, uint32_t block);

// each step of the write starts the next one from its callback
class WriteFileClosed : public dnp3::FileOperationCallback {
    void on_complete(dnp3::Nothing result) override { std::cout << "wf: success" << std::endl; }

    void on_failure(dnp3::FileError error) override { std::cout << "wf: " << dnp3::to_string(error) << std::endl; }
};

class WriteFileBlockWritten : public dnp3::FileOperationCallback {
public:
    WriteFileBlockWritten(dnp3::MasterChannel &channel, dnp3::AssociationId assoc, uint32_t handle, uint32_t block)
        : channel(channel), assoc(assoc), handle(handle), block(block)
    {
    }

    void on_complete(dnp3::Nothing result) override
    {
        try {
            if (block == 0) {
                write_hello_world_block(channel, assoc, handle, 1);
            }
            else {
                channel.close_file(assoc, handle, std::make_unique<WriteFileClosed>());
            }
        }
        catch (const std::exception &ex) {
            std::cout << "wf: " << ex.what() << std::endl;
        }
    }

    void on_failure(dnp3::FileError error) override { std::cout << "wf: " << dnp3::to_string(error) << std::endl; }

private:
    dnp3::MasterChannel &channel;
    dnp3::AssociationId assoc;
    uint32_t handle;
    uint32_t block;
};

class WriteFileOpened : public dnp3::FileOpenCallback {
public:
    WriteFileOpened(dnp3::MasterChannel &channel, dnp3::AssociationId assoc) : channel(channel), assoc(assoc) {}

    void on_complete(const dnp3::OpenFile &file) override
    {
        try {
            write_hello_world_block(channel, assoc, file.file_handle, 0);
        }
        catch (const std::exception &ex) {
            std::cout << "wf: " << ex.what() << std::endl;
        }
    }

    void on_failure(dnp3::FileError error) override { std::cout << "wf: " << dnp3::to_string(error) << std::endl; }

private:
    dnp3::MasterChannel &channel;
    dnp3::AssociationId assoc;
};

// write the 'hello world' line as block 0, then again as block 1, the last one
void write_hello_world_block(dnp3::MasterChannel &channel, dnp3::AssociationId assoc, uint32_t handle, uint32_t block)
{
    const std::string line = "hello world\n";
    const std::vector<uint8_t> data(line.begin(), line.end());
    channel.write_file_block(assoc, handle, block, block == 1, data, std::make_unique<WriteFileBlockWritten>(channel, assoc, handle, block));
}

void start_write_file(dnp3::MasterChannel &channel, dnp3::AssociationId assoc)
{
    channel.open_file(
        assoc,
        "hello_world.txt",
        0, // no authentication key
        dnp3::Permissions::none(),
        0xFFFFFFFF, // indicate that we don't know the size
        dnp3::FileMode::write,
        512, // maximum block size
        std::make_unique<WriteFileOpened>(channel, assoc)
    );
}
// ANCHOR_END: write_file

std::unique_ptr<dnp3::LinkStatusCallback> create_link_status_callback(const char *cmd)
{
    return std::make_unique<PrintResult<dnp3::LinkStatusCallback, dnp3::LinkStatusError>>(cmd);
}

void print_file_info(const dnp3::FileInfo &info)
{
    const auto permissions = [](const dnp3::PermissionSet &set) {
        return std::string(set.read ? "r" : "-") + (set.write ? "w" : "-") + (set.execute ? "x" : "-");
    };
    std::cout << "File name: " << info.file_name << std::endl;
    std::cout << "     type: " << dnp3::to_string(info.file_type) << std::endl;
    std::cout << "     size: " << info.size << std::endl;
    std::cout << "     created: " << info.time_created << std::endl;
    std::cout << "     permissions:" << std::endl;
    std::cout << "         world: " << permissions(info.permissions.world) << std::endl;
    std::cout << "         group: " << permissions(info.permissions.group) << std::endl;
    std::cout << "         owner: " << permissions(info.permissions.owner) << std::endl;
}

void print_usage()
{
    std::cout << "usage: cpp_master_example <tcp-client|udp|serial|tls-ca|tls-self-signed> [endpoint|serial-port]" << std::endl;
}
