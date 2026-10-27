// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package io.stepfunc.dnp3.examples;

import static org.joou.Unsigned.*;

import io.stepfunc.dnp3.*;
import io.stepfunc.dnp3.Runtime;
import org.joou.UByte;
import org.joou.UInteger;
import org.joou.ULong;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.ExecutionException;

/*
  Example program using the master API.

  The program initializes a master channel based on the command line argument and then enters a loop
  reading console input allowing the user to perform common tasks interactively.
*/
public class MasterExample {

  private static final int MASTER_ADDRESS = 1;
  private static final int OUTSTATION_ADDRESS = 1024;
  private static final String TCP_ENDPOINT = "127.0.0.1:20000";
  private static final String TLS_ENDPOINT = "127.0.0.1:20001";
  private static final String UDP_LOCAL_ENDPOINT = "127.0.0.1:20001";
  private static final String UDP_REMOTE_ENDPOINT = "127.0.0.1:20000";
  private static final String SERIAL_PORT = "/dev/ttyS0";
  private static final String CA_CERT = "./certs/ca_chain/ca_cert.pem";
  private static final String CA_ENTITY_CERT = "./certs/ca_chain/entity1_cert.pem";
  private static final String CA_ENTITY_KEY = "./certs/ca_chain/entity1_key.pem";
  private static final String SELF_SIGNED_PEER_CERT = "./certs/self_signed/entity2_cert.pem";
  private static final String SELF_SIGNED_ENTITY_CERT = "./certs/self_signed/entity1_cert.pem";
  private static final String SELF_SIGNED_ENTITY_KEY = "./certs/self_signed/entity1_key.pem";

  private static final List<String> TRANSPORTS =
      Arrays.asList("tcp-client", "udp", "serial", "tls-ca", "tls-self-signed");

  public static void main(String[] args) throws Exception {
    if (args.length < 1 || args.length > 2 || !TRANSPORTS.contains(args[0])) {
      System.err.println(
          "Usage: master-example <tcp-client|udp|serial|tls-ca|tls-self-signed> [endpoint|serial-port]");
      System.exit(1);
    }

    // Initialize logging with the default configuration
    // This may only be called once during program initialization
    // ANCHOR: logging_init
    Logging.configure(new LoggingConfig(), new ConsoleLogger());
    // ANCHOR_END: logging_init

    // ANCHOR: runtime_init
    Runtime runtime = new Runtime(getRuntimeConfig());
    // ANCHOR_END: runtime_init

    try {
      run(runtime, args);
    } finally {
      // ANCHOR: runtime_shutdown
      runtime.shutdown();
      // ANCHOR_END: runtime_shutdown
    }
  }

  // ANCHOR: runtime_config
  private static RuntimeConfig getRuntimeConfig() {
    return new RuntimeConfig().withNumCoreThreads(ushort(4));
  }
  // ANCHOR_END: runtime_config

  private static void run(Runtime runtime, String[] args) throws IOException {
    // create the master channel based on the command line argument
    final MasterChannel channel = createChannel(runtime, args);
    final AssociationId association =
        args[0].equals("udp")
            ? addUdpAssociation(channel, argumentOr(args, UDP_REMOTE_ENDPOINT))
            : addAssociation(channel);

    // create an event poll
    // ANCHOR: add_poll
    PollId poll =
            channel.addPoll(
                    association, Request.classRequest(false, true, true, true), Duration.ofSeconds(5));
    // ANCHOR_END: add_poll

    // enable communications
    channel.enable();

    runConsole(channel, association, poll);
  }

  private static MasterChannel createChannel(Runtime runtime, String[] args) {
    final String transport = args[0];
    if (transport.equals("tcp-client")) {
      return createTcpChannel(runtime, argumentOr(args, TCP_ENDPOINT));
    } else if (transport.equals("udp")) {
      return createUdpChannel(runtime);
    } else if (transport.equals("serial")) {
      return createSerialChannel(runtime, argumentOr(args, SERIAL_PORT));
    } else if (transport.equals("tls-ca")) {
      return createTlsChannel(runtime, argumentOr(args, TLS_ENDPOINT), getTlsCaConfig());
    } else {
      return createTlsChannel(runtime, argumentOr(args, TLS_ENDPOINT), getTlsSelfSignedConfig());
    }
  }

  private static MasterChannel createTcpChannel(Runtime runtime, String endpoint) {
    // ANCHOR: create_tcp_channel
    MasterChannel channel =
            MasterChannel.createTcpChannel(
                    runtime,
                    LinkErrorMode.CLOSE,
                    getMasterChannelConfig(),
                    new EndpointList(endpoint),
                    new ConnectStrategy(),
                    state -> System.out.println("client state: " + state));
    // ANCHOR_END: create_tcp_channel
    return channel;
  }

  private static MasterChannel createUdpChannel(Runtime runtime) {
    // ANCHOR: create_udp_channel
    MasterChannel channel =
            MasterChannel.createUdpChannel(
                    runtime,
                    getMasterChannelConfig(),
                    UDP_LOCAL_ENDPOINT,
                    LinkReadMode.DATAGRAM,
                    Duration.ofSeconds(5)
            );
    // ANCHOR_END: create_udp_channel
    return channel;
  }

  private static MasterChannel createSerialChannel(Runtime runtime, String port) {
    // ANCHOR: create_serial_channel
    MasterChannel channel =
            MasterChannel.createSerialChannel(
                    runtime,
                    getMasterChannelConfig(),
                    port,
                    new SerialSettings(), // 9600 baud, 8 data bits, 1 stop bit, no parity, no flow control
                    Duration.ofSeconds(1), // try to open the port every second
                    state -> System.out.println("port state: " + state)
            );
    // ANCHOR_END: create_serial_channel
    return channel;
  }

  private static MasterChannel createTlsChannel(Runtime runtime, String endpoint, TlsClientConfig config) {
    // ANCHOR: create_tls_channel
    MasterChannel channel =
            MasterChannel.createTlsChannel(
                    runtime,
                    LinkErrorMode.CLOSE,
                    getMasterChannelConfig(),
                    new EndpointList(endpoint),
                    new ConnectStrategy(),
                    state -> System.out.println("client state: " + state),
                    config);
    // ANCHOR_END: create_tls_channel
    return channel;
  }

  // ANCHOR: master_channel_config
  private static MasterChannelConfig getMasterChannelConfig() {
    MasterChannelConfig config = new MasterChannelConfig(ushort(MASTER_ADDRESS));
    config.decodeLevel.application = AppDecodeLevel.OBJECT_VALUES;
    return config;
  }
  // ANCHOR_END: master_channel_config

  private static TlsClientConfig getTlsCaConfig() {
    // ANCHOR: tls_ca_chain_config
    TlsClientConfig config =
            new TlsClientConfig(
                    "test.com",
                    CA_CERT,
                    CA_ENTITY_CERT,
                    CA_ENTITY_KEY,
                    "" // no password
            );
    // ANCHOR_END: tls_ca_chain_config
    return config;
  }

  private static TlsClientConfig getTlsSelfSignedConfig() {
    // ANCHOR: tls_self_signed_config
    TlsClientConfig config =
            new TlsClientConfig(
                    "test.com",
                    SELF_SIGNED_PEER_CERT,
                    SELF_SIGNED_ENTITY_CERT,
                    SELF_SIGNED_ENTITY_KEY,
                    "" // no password
            ).withCertificateMode(CertificateMode.SELF_SIGNED);
    // ANCHOR_END: tls_self_signed_config
    return config;
  }

  private static AssociationId addAssociation(MasterChannel channel) {
    // ANCHOR: association_create
    AssociationId association =
        channel.addAssociation(
            ushort(OUTSTATION_ADDRESS),
            getAssociationConfig(),
            new ExampleReadHandler(),
            new ExampleAssociationHandler(),
            new ExampleAssociationInformation());
    // ANCHOR_END: association_create
    return association;
  }

  private static AssociationId addUdpAssociation(MasterChannel channel, String remoteEndpoint) {
    // ANCHOR: association_create_udp
    AssociationId association =
        channel.addUdpAssociation(
            ushort(OUTSTATION_ADDRESS),
            remoteEndpoint,
            getAssociationConfig(),
            new ExampleReadHandler(),
            new ExampleAssociationHandler(),
            new ExampleAssociationInformation());
    // ANCHOR_END: association_create_udp
    return association;
  }

  // ANCHOR: association_config
  private static AssociationConfig getAssociationConfig() {
    return new AssociationConfig(
            // disable unsolicited first (Class 1/2/3)
            EventClasses.all(),
            // after the integrity poll, enable unsolicited (Class 1/2/3)
            EventClasses.all(),
            // perform startup integrity poll with Class 1/2/3/0
            Classes.all(),
            // don't automatically scan Class 1/2/3 when the corresponding IIN bit is asserted
            EventClasses.none())
      .withAutoTimeSync(AutoTimeSync.LAN)
      .withKeepAliveTimeout(Duration.ofSeconds(60));
  }
  // ANCHOR_END: association_config

  private static void runConsole(MasterChannel channel, AssociationId association, PollId poll) throws IOException {
    final BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));

    printHelp();

    // exit on "x" or at the end of input
    while (true) {
      final String line = reader.readLine();
      if (line == null || line.equals("x")) {
        return;
      } else if (line.equals("help")) {
        printHelp();
      } else {
        try {
          System.out.println(line + ": " + runOneCommand(channel, association, poll, line));
        } catch (ExecutionException ex) {
          // the command's CompletionStage failed
          System.out.println(line + ": " + ex.getCause().getMessage());
        } catch (Exception ex) {
          System.out.println(line + ": " + ex.getMessage());
        }
      }
    }
  }

  // runs one command, waits for its result, and returns the outcome
  private static String runOneCommand(MasterChannel channel, AssociationId association, PollId poll, String command) throws Exception {
    final String success = "success";
    switch (command) {
      case "enable":
        channel.enable();
        return success;
      case "disable":
        channel.disable();
        return success;
      case "dln":
        channel.setDecodeLevel(DecodeLevel.nothing());
        return success;
      case "dlv":
        channel.setDecodeLevel(DecodeLevel.nothing().withApplication(AppDecodeLevel.OBJECT_VALUES));
        return success;
      case "rao":
      {
        Request request = new Request();
        request.addAllObjectsHeader(Variation.GROUP40_VAR0);
        channel.read(association, request).toCompletableFuture().get();
        return success;
      }
      case "rmo":
      {
        Request request = new Request();
        request.addAllObjectsHeader(Variation.GROUP10_VAR0);
        request.addAllObjectsHeader(Variation.GROUP40_VAR0);
        channel.read(association, request).toCompletableFuture().get();
        return success;
      }
      case "rco":
      {
        Request request = new Request();
        request.addAllObjectsHeader(Variation.GROUP20_VAR0);
        request.addAllObjectsHeader(Variation.GROUP21_VAR0);
        channel.read(association, request).toCompletableFuture().get();
        return success;
      }
      case "cmd":
      {
        // ANCHOR: assoc_control
        CommandSet commands = new CommandSet();
        Group12Var1 control = Group12Var1.fromCode(ControlCode.fromOpType(OpType.LATCH_ON));
        commands.addG12V1U16(ushort(3), control);
        channel
                .operate(association, CommandMode.SELECT_BEFORE_OPERATE, commands)
                .toCompletableFuture()
                .get();
        // ANCHOR_END: assoc_control
        return success;
      }
      case "cna":
      {
        // ANCHOR: assoc_control_no_ack
        CommandSet commands = new CommandSet();
        Group12Var1 control = Group12Var1.fromCode(ControlCode.fromOpType(OpType.LATCH_ON));
        commands.addG12V1U16(ushort(3), control);
        channel.operateNoAck(association, commands).toCompletableFuture().get();
        // ANCHOR_END: assoc_control_no_ack
        // the request was sent: the outstation doesn't answer
        return "sent";
      }
      case "evt":
        // the poll runs later, and its data goes to the read handler
        channel.demandPoll(poll);
        return "poll demanded";
      case "lts":
        channel.synchronizeTime(association, TimeSyncMode.LAN).toCompletableFuture().get();
        return success;
      case "nts":
        channel.synchronizeTime(association, TimeSyncMode.NON_LAN).toCompletableFuture().get();
        return success;
      case "wad":
      {
        // ANCHOR: write_dead_bands
        WriteDeadBandRequest request = new WriteDeadBandRequest();
        request.addG34v1U8(ubyte(3), ushort(5));
        request.addG34v3U16(ushort(4), 2.5f);
        channel.writeDeadBands(association, request).toCompletableFuture().get();
        // ANCHOR_END: write_dead_bands
        return success;
      }
      case "fat":
      {
        // ANCHOR: freeze_at_time
        Request request = new Request();
        // freeze all the counters once per day relative to the beginning of the current hour
        request.addTimeAndInterval(ulong(0), uint(86400000));
        // apply this schedule to all counters
        request.addAllObjectsHeader(Variation.GROUP20_VAR0);
        channel.sendAndExpectEmptyResponse(association, FunctionCode.FREEZE_AT_TIME, request).toCompletableFuture().get();
        // ANCHOR_END: freeze_at_time
        return success;
      }
      case "ifz":
      {
        // copy every counter to its frozen counter
        Request request = new Request();
        request.addAllObjectsHeader(Variation.GROUP20_VAR0);
        channel.sendAndExpectEmptyResponse(association, FunctionCode.IMMEDIATE_FREEZE, request).toCompletableFuture().get();
        return success;
      }
      case "fcl":
      {
        // copy every counter to its frozen counter, then clear the counter
        Request request = new Request();
        request.addAllObjectsHeader(Variation.GROUP20_VAR0);
        channel.sendAndExpectEmptyResponse(association, FunctionCode.FREEZE_CLEAR, request).toCompletableFuture().get();
        return success;
      }
      case "rda":
      {
        // ANCHOR: read_attributes
        Request request = new Request();
        request.addSpecificAttribute(AttributeVariations.ALL_ATTRIBUTES_REQUEST, ubyte(0));
        channel.read(association, request).toCompletableFuture().get();
        // ANCHOR_END: read_attributes
        return success;
      }
      case "wda":
      {
        // ANCHOR: write_attribute
        Request request = new Request();
        request.addStringAttribute(AttributeVariations.USER_ASSIGNED_LOCATION, ubyte(0), "Mt. Olympus");
        channel.sendAndExpectEmptyResponse(association, FunctionCode.WRITE, request).toCompletableFuture().get();
        // ANCHOR_END: write_attribute
        return success;
      }
      case "ral":
      {
        Request request = new Request();
        request.addSpecificAttribute(AttributeVariations.LIST_OF_VARIATIONS, ubyte(0));
        channel.read(association, request).toCompletableFuture().get();
        return success;
      }
      case "crt":
      {
        Duration delay = channel.coldRestart(association).toCompletableFuture().get();
        System.out.println("restart delay: " + delay);
        return success;
      }
      case "wrt":
      {
        Duration delay = channel.warmRestart(association).toCompletableFuture().get();
        System.out.println("restart delay: " + delay);
        return success;
      }
      case "rd":
      {
        // ANCHOR: read_directory
        List<FileInfo> items = channel
                .readDirectory(association, ".", DirReadConfig.defaults())
                .toCompletableFuture().get();
        for (FileInfo info : items) {
          printFileInfo(info);
        }
        // ANCHOR_END: read_directory
        return success;
      }
      case "gfi":
      {
        // ANCHOR: get_file_info
        FileInfo info = channel.getFileInfo(association, ".").toCompletableFuture().get();
        printFileInfo(info);
        // ANCHOR_END: get_file_info
        return success;
      }
      case "rf":
      {
        // ANCHOR: read_file
        // this reads the root "directory" file but doesn't parse it
        channel.readFile(association, ".", FileReadConfig.defaults(), new FileLogger());
        // ANCHOR_END: read_file
        return "started (the file reader reports the outcome)";
      }
      case "wf":
      {
        // ANCHOR: write_file
        OpenFile file = channel.openFile(
                association,
                "hello_world.txt",
                uint(0), // no authentication key
                Permissions.none(),
                uint(0xFFFFFFFFL), // indicate that we don't know the size
                FileMode.WRITE,
                ushort(512)
        ).toCompletableFuture().get();

        List<UByte> line = new ArrayList<>();
        for (byte b : "hello world\n".getBytes(StandardCharsets.UTF_8)) {
          line.add(ubyte(b));
        }

        // write the 'hello world' line to the file twice
        channel.writeFileBlock(association, file.fileHandle, uint(0), false, line).toCompletableFuture().get();
        channel.writeFileBlock(association, file.fileHandle, uint(1), true, line).toCompletableFuture().get();
        channel.closeFile(association, file.fileHandle).toCompletableFuture().get();
        // ANCHOR_END: write_file
        return success;
      }
      case "lsr":
        channel.checkLinkStatus(association).toCompletableFuture().get();
        return success;
      default:
        return "unknown command, type help for a list";
    }
  }

  private static void printHelp() {
    System.out.println("Commands:");
    System.out.println("  help - print this list");
    System.out.println("  x - exit");
    System.out.println("  enable / disable - enable or disable the channel");
    System.out.println("  dln / dlv - decode nothing / decode object values");
    System.out.println("  rao - read analog output statuses");
    System.out.println("  rmo - read binary and analog output statuses");
    System.out.println("  rco - read counters and frozen counters");
    System.out.println("  cmd - select and operate binary output 3 (latch on)");
    System.out.println("  cna - operate binary output 3 without an answer");
    System.out.println("  evt - demand the event poll");
    System.out.println("  lts / nts - synchronize the time (LAN / non-LAN)");
    System.out.println("  wad - write analog input deadbands");
    System.out.println("  ifz - freeze all counters");
    System.out.println("  fcl - freeze and clear all counters");
    System.out.println("  fat - freeze all counters once a day");
    System.out.println("  rda - read all device attributes");
    System.out.println("  wda - write the user-assigned location attribute");
    System.out.println("  ral - read the list of attribute variations");
    System.out.println("  crt / wrt - cold / warm restart");
    System.out.println("  rd - read the root directory");
    System.out.println("  gfi - get information about the root directory");
    System.out.println("  rf - read the root directory as a file");
    System.out.println("  wf - write hello_world.txt");
    System.out.println("  lsr - check the link status");
  }

  private static void printFileInfo(FileInfo info) {
    System.out.println("File name: " + info.fileName);
    System.out.println("     type: " + info.fileType);
    System.out.println("     size: " + info.size);
    System.out.println("     created: " + info.timeCreated);
    System.out.println("     permissions:");
    System.out.println("         world: " + formatPermissions(info.permissions.world));
    System.out.println("         group: " + formatPermissions(info.permissions.group));
    System.out.println("         owner: " + formatPermissions(info.permissions.owner));
  }

  private static String formatPermissions(PermissionSet permissions) {
    return (permissions.read ? "r" : "-") + (permissions.write ? "w" : "-") + (permissions.execute ? "x" : "-");
  }

  // the optional argument after the transport, or the default
  private static String argumentOr(String[] args, String defaultValue) {
    return args.length > 1 ? args[1] : defaultValue;
  }
}

// ANCHOR: logging_interface
// callback interface used to receive log messages
class ConsoleLogger implements Logger {

  @Override
  public void onMessage(LogLevel level, String message) {
    System.out.print(message);
  }
}
// ANCHOR_END: logging_interface

// ANCHOR: read_handler
class ExampleReadHandler implements ReadHandler {

  @Override
  public void beginFragment(ReadType readType, ResponseHeader header) {
    System.out.println("begin fragment: " + readType + " (broadcast: " + header.iin.iin1.broadcast + ")");
  }

  @Override
  public void endFragment(ReadType readType, ResponseHeader header) {
    System.out.println("end fragment");
  }

  @Override
  public void handleBinaryInput(HeaderInfo info, List<BinaryInput> values) {
    printHeader("Binary inputs", info);
    values.forEach(x -> printPoint("BI", x.index.intValue(), x.value, x.flags, x.time));
  }

  @Override
  public void handleDoubleBitBinaryInput(HeaderInfo info, List<DoubleBitBinaryInput> values) {
    printHeader("Double-bit binary inputs", info);
    values.forEach(x -> printPoint("DBBI", x.index.intValue(), x.value, x.flags, x.time));
  }

  @Override
  public void handleBinaryOutputStatus(HeaderInfo info, List<BinaryOutputStatus> values) {
    printHeader("Binary output statuses", info);
    values.forEach(x -> printPoint("BOS", x.index.intValue(), x.value, x.flags, x.time));
  }

  @Override
  public void handleCounter(HeaderInfo info, List<Counter> values) {
    printHeader("Counters", info);
    values.forEach(x -> printPoint("C", x.index.intValue(), x.value, x.flags, x.time));
  }

  @Override
  public void handleFrozenCounter(HeaderInfo info, List<FrozenCounter> values) {
    printHeader("Frozen counters", info);
    values.forEach(x -> printPoint("FC", x.index.intValue(), x.value, x.flags, x.time));
  }

  @Override
  public void handleAnalogInput(HeaderInfo info, List<AnalogInput> values) {
    printHeader("Analog inputs", info);
    values.forEach(x -> printPoint("AI", x.index.intValue(), x.value, x.flags, x.time));
  }

  @Override
  public void handleFrozenAnalogInput(HeaderInfo info, List<FrozenAnalogInput> values) {
    printHeader("Frozen analog inputs", info);
    values.forEach(x -> printPoint("FAI", x.index.intValue(), x.value, x.flags, x.time));
  }

  @Override
  public void handleAnalogInputDeadBand(HeaderInfo info, List<AnalogInputDeadBand> values) {
    printHeader("Analog input deadbands", info);
    values.forEach(x -> System.out.println("AIDB " + x.index + ": value=" + x.value));
  }

  @Override
  public void handleAnalogOutputStatus(HeaderInfo info, List<AnalogOutputStatus> values) {
    printHeader("Analog output statuses", info);
    values.forEach(x -> printPoint("AOS", x.index.intValue(), x.value, x.flags, x.time));
  }

  @Override
  public void handleBinaryOutputCommandEvent(HeaderInfo info, List<BinaryOutputCommandEvent> values) {
    printHeader("Binary output command events", info);
    values.forEach(x -> System.out.println(
        "BOCE " + x.index + ": commanded state=" + x.commandedState + " status=" + x.status + " time=" + formatTime(x.time)));
  }

  @Override
  public void handleAnalogOutputCommandEvent(HeaderInfo info, List<AnalogOutputCommandEvent> values) {
    printHeader("Analog output command events", info);
    values.forEach(x -> System.out.println(
        "AOCE " + x.index + ": commanded value=" + x.commandedValue + " status=" + x.status + " time=" + formatTime(x.time)));
  }

  @Override
  public void handleUnsignedInteger(HeaderInfo info, List<UnsignedInteger> values) {
    printHeader("Unsigned integers", info);
    values.forEach(x -> System.out.println("UI " + x.index + ": value=" + x.value));
  }

  @Override
  public void handleOctetString(HeaderInfo info, List<OctetString> values) {
    printHeader("Octet strings", info);
    values.forEach(x -> System.out.println("OS " + x.index + ": value=" + formatBytes(x.value)));
  }

  @Override
  public void handleStringAttr(HeaderInfo info, StringAttr attr, UByte set, UByte variation, String value) {
    printHeader("Device attributes", info);
    printAttribute(set, variation, value);
  }

  @Override
  public void handleVariationListAttr(HeaderInfo info, VariationListAttr attr, UByte set, UByte variation, List<AttrItem> value) {
    printHeader("Device attributes", info);
    List<String> variations = new ArrayList<>();
    value.forEach(item -> variations.add(item.variation + (item.properties.isWritable ? " (writable)" : "")));
    printAttribute(set, variation, variations);
  }

  @Override
  public void handleUintAttr(HeaderInfo info, UintAttr attr, UByte set, UByte variation, UInteger value) {
    printHeader("Device attributes", info);
    printAttribute(set, variation, value);
  }

  @Override
  public void handleBoolAttr(HeaderInfo info, BoolAttr attr, UByte set, UByte variation, boolean value) {
    printHeader("Device attributes", info);
    printAttribute(set, variation, value);
  }

  @Override
  public void handleIntAttr(HeaderInfo info, IntAttr attr, UByte set, UByte variation, int value) {
    printHeader("Device attributes", info);
    printAttribute(set, variation, value);
  }

  @Override
  public void handleTimeAttr(HeaderInfo info, TimeAttr attr, UByte set, UByte variation, ULong value) {
    printHeader("Device attributes", info);
    printAttribute(set, variation, value);
  }

  @Override
  public void handleFloatAttr(HeaderInfo info, FloatAttr attr, UByte set, UByte variation, double value) {
    printHeader("Device attributes", info);
    printAttribute(set, variation, value);
  }

  @Override
  public void handleOctetStringAttr(HeaderInfo info, OctetStringAttr attr, UByte set, UByte variation, List<UByte> value) {
    printHeader("Device attributes", info);
    printAttribute(set, variation, formatBytes(value));
  }

  @Override
  public void handleBitStringAttr(HeaderInfo info, BitStringAttr attr, UByte set, UByte variation, List<UByte> value) {
    printHeader("Device attributes", info);
    printAttribute(set, variation, formatBytes(value));
  }

  @Override
  public void handleAbsTime(HeaderInfo info, Timestamp time) {
    printHeader("Absolute time", info);
    System.out.println("time=" + time.value);
  }

  private static void printHeader(String name, HeaderInfo info) {
    System.out.println(name + " (" + info.qualifier + ", " + info.variation + ")");
  }

  private static void printPoint(String name, int index, Object value, Flags flags, Timestamp time) {
    System.out.printf("%s %d: value=%s flags=0x%02X time=%s%n", name, index, value, flags.value.intValue(), formatTime(time));
  }

  private static String formatTime(Timestamp time) {
    switch (time.quality) {
      case SYNCHRONIZED_TIME:
        return time.value + " (synchronized)";
      case UNSYNCHRONIZED_TIME:
        return time.value + " (unsynchronized)";
      default:
        return "none";
    }
  }

  private static void printAttribute(UByte set, UByte variation, Object value) {
    System.out.println("attribute set=" + set + " variation=" + variation + " value=" + value);
  }

  private static String formatBytes(List<UByte> bytes) {
    List<String> hex = new ArrayList<>();
    bytes.forEach(b -> hex.add(String.format("%02X", b.intValue())));
    return hex.toString();
  }
}
// ANCHOR_END: read_handler

// ANCHOR: association_handler
class ExampleAssociationHandler implements AssociationHandler {
  @Override
  public UtcTimestamp getCurrentTime() {
    return UtcTimestamp.valid(ulong(System.currentTimeMillis()));
  }
}
// ANCHOR_END: association_handler

// ANCHOR: association_information
class ExampleAssociationInformation implements AssociationInformation {
  @Override
  public void taskStart(TaskType taskType, FunctionCode fc, UByte seq) {}

  @Override
  public void taskSuccess(TaskType taskType, FunctionCode fc, UByte seq) {}

  @Override
  public void taskFail(TaskType taskType, TaskError error) {}

  @Override
  public void unsolicitedResponse(boolean isDuplicate, UByte seq) {}
}
// ANCHOR_END: association_information

// ANCHOR: file_logger
class FileLogger implements FileReader {
  @Override
  public boolean opened(UInteger size) {
    System.out.println("File opened - size: " + size);
    return true;
  }

  @Override
  public boolean blockReceived(UInteger blockNum, List<UByte> data) {
    System.out.println("Received block " + blockNum + " with size: " + data.size());
    return true;
  }

  @Override
  public void aborted(FileError error) {
    System.out.println("rf: aborted: " + error);
  }

  @Override
  public void completed() {
    System.out.println("rf: completed");
  }
}
// ANCHOR_END: file_logger
