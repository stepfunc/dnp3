package io.stepfunc.dnp3.examples;

import static org.joou.Unsigned.*;

import io.stepfunc.dnp3.*;
import io.stepfunc.dnp3.Runtime;
import org.joou.UByte;
import org.joou.UInteger;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.ArrayList;
import java.util.List;

// ANCHOR: logging_interface
// callback interface used to receive log messages
class ConsoleLogger implements Logger {

  @Override
  public void onMessage(LogLevel level, String message) {
    System.out.print(message);
  }
}
// ANCHOR_END: logging_interface

class TestClientStateListener implements ClientStateListener {

  @Override
  public void onChange(ClientState state) {
    System.out.println(state);
  }
}

// ANCHOR: read_handler
class TestReadHandler implements ReadHandler {

    @Override
    public void beginFragment(ReadType readType, ResponseHeader header) {
      System.out.println("Beginning fragment (broadcast: " + header.iin.iin1.broadcast + ")");
    }

    @Override
    public void endFragment(ReadType readType, ResponseHeader header) {
      System.out.println("End fragment");
    }

    @Override
    public void handleBinaryInput(HeaderInfo info, List<BinaryInput> it) {
      System.out.println("Binary Inputs:");
      System.out.println("Qualifier: " + info.qualifier);
      System.out.println("Variation: " + info.variation);

      it.forEach(
          val -> {
            System.out.println(
                "BI "
                    + val.index
                    + ": Value="
                    + val.value
                    + " Flags="
                    + val.flags.value
                    + " Time="
                    + val.time.value
                    + " ("
                    + val.time.quality
                    + ")");
          });
    }

    @Override
    public void handleDoubleBitBinaryInput(HeaderInfo info, List<DoubleBitBinaryInput> it) {
      System.out.println("Double Bit Binary Inputs:");
      System.out.println("Qualifier: " + info.qualifier);
      System.out.println("Variation: " + info.variation);

      it.forEach(
          val -> {
            System.out.println(
                "DBBI "
                    + val.index
                    + ": Value="
                    + val.value
                    + " Flags="
                    + val.flags.value
                    + " Time="
                    + val.time.value
                    + " ("
                    + val.time.quality
                    + ")");
          });
    }

    @Override
    public void handleBinaryOutputStatus(HeaderInfo info, List<BinaryOutputStatus> it) {
      System.out.println("Binary Output Statuses:");
      System.out.println("Qualifier: " + info.qualifier);
      System.out.println("Variation: " + info.variation);

      it.forEach(
          val -> {
            System.out.println(
                "BOS "
                    + val.index
                    + ": Value="
                    + val.value
                    + " Flags="
                    + val.flags.value
                    + " Time="
                    + val.time.value
                    + " ("
                    + val.time.quality
                    + ")");
          });
    }

    @Override
    public void handleCounter(HeaderInfo info, List<Counter> it) {
      System.out.println("Counters:");
      System.out.println("Qualifier: " + info.qualifier);
      System.out.println("Variation: " + info.variation);

      it.forEach(
          val -> {
            System.out.println(
                "Counter "
                    + val.index
                    + ": Value="
                    + val.value
                    + " Flags="
                    + val.flags.value
                    + " Time="
                    + val.time.value
                    + " ("
                    + val.time.quality
                    + ")");
          });
    }

    @Override
    public void handleFrozenCounter(HeaderInfo info, List<FrozenCounter> it) {
      System.out.println("Frozen Counters:");
      System.out.println("Qualifier: " + info.qualifier);
      System.out.println("Variation: " + info.variation);

      it.forEach(
          val -> {
            System.out.println(
                "Frozen Counter "
                    + val.index
                    + ": Value="
                    + val.value
                    + " Flags="
                    + val.flags.value
                    + " Time="
                    + val.time.value
                    + " ("
                    + val.time.quality
                    + ")");
          });
    }

    @Override
    public void handleAnalogInput(HeaderInfo info, List<AnalogInput> it) {
      System.out.println("Analog Inputs:");
      System.out.println("Qualifier: " + info.qualifier);
      System.out.println("Variation: " + info.variation);

      it.forEach(
          val -> {
            System.out.println(
                "AI "
                    + val.index
                    + ": Value="
                    + val.value
                    + " Flags="
                    + val.flags.value
                    + " Time="
                    + val.time.value
                    + " ("
                    + val.time.quality
                    + ")");
          });
    }

    @Override
    public void handleAnalogOutputStatus(HeaderInfo info, List<AnalogOutputStatus> it) {
      System.out.println("Analog Output Statuses:");
      System.out.println("Qualifier: " + info.qualifier);
      System.out.println("Variation: " + info.variation);

      it.forEach(
          val -> {
            System.out.println(
                "AOS "
                    + val.index
                    + ": Value="
                    + val.value
                    + " Flags="
                    + val.flags.value
                    + " Time="
                    + val.time.value
                    + " ("
                    + val.time.quality
                    + ")");
          });
    }

    @Override
    public void handleOctetString(HeaderInfo info, List<OctetString> it) {
      System.out.println("Octet Strings:");
      System.out.println("Qualifier: " + info.qualifier);
      System.out.println("Variation: " + info.variation);

      it.forEach(
          val -> {
            System.out.print("Octet String " + val.index + ": Value=");
            val.value.forEach(
                b -> System.out.print(String.format("%02X", b.byteValue()) + " "));
            System.out.println();
          });
    }

    @Override
    public void handleStringAttr(HeaderInfo info, StringAttr attr, UByte set, UByte variation, String value) {
      System.out.printf("String attribute: %s set: %d var: %d value: %s%n", attr, set.intValue(), variation.intValue(), value);
    }
}
// ANCHOR_END: read_handler

// ANCHOR: association_handler
class TestAssociationHandler implements AssociationHandler {
  @Override
  public UtcTimestamp getCurrentTime() {
    return UtcTimestamp.valid(ulong(System.currentTimeMillis()));
  }
}
// ANCHOR_END: association_handler

// ANCHOR: association_information
class TestAssociationInformation implements AssociationInformation {
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
class LoggingFileReader implements FileReader {
  @Override
  public boolean opened(UInteger size) {
    System.out.println("Opened file - size: " + size);
    return true;
  }

  @Override
  public boolean blockReceived(UInteger blockNum, List<UByte> data) {
    System.out.println("Received file block: " + blockNum);
    return true;
  }

  @Override
  public void aborted(FileError error) {
    System.out.println("Aborted file transfer: " + error);
  }

  @Override
  public void completed() {
    System.out.println("Completed file transfer");
  }
}
// ANCHOR_END: file_logger

public class MasterExample {

  // ANCHOR: master_channel_config
  private static MasterChannelConfig getMasterChannelConfig() {
    MasterChannelConfig config = new MasterChannelConfig(ushort(1));
    config.decodeLevel.application = AppDecodeLevel.OBJECT_VALUES;
    return config;
  }
  // ANCHOR_END: master_channel_config

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

  // ANCHOR: runtime_config
  private static RuntimeConfig getRuntimeConfig() {
    return new RuntimeConfig().withNumCoreThreads(ushort(4));
  }
  // ANCHOR_END: runtime_config

  private static TlsClientConfig getTlsSelfSignedConfig() {
    // ANCHOR: tls_self_signed_config
    TlsClientConfig config =
            new TlsClientConfig(
                    "test.com",
                    "./certs/self_signed/entity2_cert.pem",
                    "./certs/self_signed/entity1_cert.pem",
                    "./certs/self_signed/entity1_key.pem",
                    "" // no password
            ).withCertificateMode(CertificateMode.SELF_SIGNED);
    // ANCHOR_END: tls_self_signed_config
    return config;
  }

  private static TlsClientConfig getTlsCAConfig() {
    // ANCHOR: tls_ca_chain_config
    TlsClientConfig config =
            new TlsClientConfig(
                    "test.com",
                    "./certs/ca_chain/ca_cert.pem",
                    "./certs/ca_chain/entity1_cert.pem",
                    "./certs/ca_chain/entity1_key.pem",
                    "" // no password
            );
    // ANCHOR_END: tls_ca_chain_config
    return config;
  }

  private static void runTcp(Runtime runtime) throws Exception {
    // ANCHOR: create_tcp_channel
    MasterChannel channel =
            MasterChannel.createTcpChannel(
                    runtime,
                    LinkErrorMode.CLOSE,
                    getMasterChannelConfig(),
                    new EndpointList("127.0.0.1:20000"),
                    new ConnectStrategy(),
                    new TestClientStateListener());
    // ANCHOR_END: create_tcp_channel

    try {
      runChannel(channel);
    }
    finally {
      channel.shutdown();
    }
  }

  private static void runTls(Runtime runtime, TlsClientConfig tlsConfig) throws Exception {
    // ANCHOR: create_tls_channel
    MasterChannel channel =
            MasterChannel.createTlsChannel(
                    runtime,
                    LinkErrorMode.CLOSE,
                    getMasterChannelConfig(),
                    new EndpointList("127.0.0.1:20001"),
                    new ConnectStrategy(),
                    new TestClientStateListener(),
                    tlsConfig);
    // ANCHOR_END: create_tls_channel

    try {
      runChannel(channel);
    }
    finally {
      channel.shutdown();
    }
  }

  private static void runUdp(Runtime runtime) throws Exception {
    // ANCHOR: create_udp_channel
    MasterChannel channel =
            MasterChannel.createUdpChannel(
                    runtime,
                    getMasterChannelConfig(),
                    "127.0.0.1:20001",
                    LinkReadMode.DATAGRAM,
                    Duration.ofSeconds(5)
            );
    // ANCHOR_END: create_udp_channel

    try {
      AssociationId association =
              channel.addUdpAssociation(
                      ushort(1024),
                      "127.0.0.1:20000",
                      getAssociationConfig(),
                      new TestReadHandler(),
                      new TestAssociationHandler(),
                      new TestAssociationInformation());

      runAssociation(channel, association);
    }
    finally {
      channel.shutdown();
    }
  }

  private static void runSerial(Runtime runtime) throws Exception {
    // ANCHOR: create_serial_channel
    MasterChannel channel =
            MasterChannel.createSerialChannel(
                    runtime,
                    getMasterChannelConfig(),
                    "/dev/pts/4", // replace with a real port
                    new SerialSettings(),
                    Duration.ofSeconds(5),
                    state -> System.out.println("Port state change: " + state)
            );
    // ANCHOR_END: create_serial_channel

    try {
      runChannel(channel);
    }
    finally {
      channel.shutdown();
    }
  }

  public static void main(String[] args) throws Exception {
    // Initialize logging with the default configuration
    // This may only be called once during program initialization
    // ANCHOR: logging_init
    Logging.configure(new LoggingConfig(), new ConsoleLogger());
    // ANCHOR_END: logging_init

    // ANCHOR: runtime
    Runtime runtime = new Runtime(getRuntimeConfig());
    // ANCHOR_END: runtime

    try {
      run(runtime, args);
    } finally {
      // ANCHOR: runtime_shutdown
      runtime.shutdown();
      // ANCHOR_END: runtime_shutdown
    }
  }

  private static void run(Runtime runtime, String[] args) throws Exception {
    if(args.length != 1) {
      System.err.println("You must specify a transport");
      System.err.println("Usage: master-example <transport> (tcp, serial, tls-ca, tls-self-signed)");
      return;
    }

    final String type = args[0];
    switch(type) {
      case "tcp":
        runTcp(runtime);
        break;
      case "udp":
        runUdp(runtime);
        break;
      case "serial":
        runSerial(runtime);
        break;
      case "tls-ca":
        runTls(runtime, getTlsCAConfig());
        break;
      case "tls-self-signed":
        runTls(runtime, getTlsSelfSignedConfig());
        break;
      default:
        System.err.printf("Unknown transport: %s%n", type);
    }
  }

  private static void runOneCommand(MasterChannel channel, AssociationId association, PollId poll, String command) throws Exception {
    switch (command) {
      case "enable":
        channel.enable();
        break;
      case "disable":
        channel.disable();
        break;
      case "dln":
        channel.setDecodeLevel(DecodeLevel.nothing());
        break;
      case "dlv":
        channel.setDecodeLevel(DecodeLevel.nothing().withApplication(AppDecodeLevel.OBJECT_VALUES));
        break;
      case "rao":
      {
        Request request = new Request();
        request.addAllObjectsHeader(Variation.GROUP40_VAR0);
        channel.read(association, request).toCompletableFuture().get();
        break;
      }
      case "rmo":
      {
        Request request = new Request();
        request.addAllObjectsHeader(Variation.GROUP10_VAR0);
        request.addAllObjectsHeader(Variation.GROUP40_VAR0);
        channel.read(association, request).toCompletableFuture().get();
        break;
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
        break;
      }
      case "evt":
        channel.demandPoll(poll);
        break;
      case "lts":
      {
        channel.synchronizeTime(association, TimeSyncMode.LAN).toCompletableFuture().get();
        break;
      }
      case "nts":
      {
        channel.synchronizeTime(association, TimeSyncMode.NON_LAN).toCompletableFuture().get();
        break;
      }
      case "wad":
      {
        WriteDeadBandRequest request = new WriteDeadBandRequest();
        request.addG34v1U8(ubyte(3),ushort(5));
        request.addG34v3U16(ushort(5), 2.5f);
        channel.writeDeadBands(association, request).toCompletableFuture().get();
        break;
      }
      case "fat":
      {
        Request request = new Request();
        request.addTimeAndInterval(ulong(0), uint(86400000));
        request.addAllObjectsHeader(Variation.GROUP20_VAR0);
        channel.sendAndExpectEmptyResponse(association, FunctionCode.FREEZE_AT_TIME, request).toCompletableFuture().get();
        break;
      }
      case "rda":
      {
        // ANCHOR: read_attributes
        Request request = new Request();
        request.addSpecificAttribute(AttributeVariations.ALL_ATTRIBUTES_REQUEST, ubyte(0));
        channel.read(association, request).toCompletableFuture().get();
        // ANCHOR_END: read_attributes
        break;
      }
      case "wda":
      {
        // ANCHOR: write_attribute
        Request request = new Request();
        request.addStringAttribute(AttributeVariations.USER_ASSIGNED_LOCATION, ubyte(0), "Mt. Olympus");
        channel.sendAndExpectEmptyResponse(association, FunctionCode.WRITE, request).toCompletableFuture().get();
        // ANCHOR_END: write_attribute
        break;
      }
      case "ral":
      {
        Request request = new Request();
        request.addSpecificAttribute(AttributeVariations.LIST_OF_VARIATIONS, ubyte(0));
        channel.read(association, request).toCompletableFuture().get();
        break;
      }
      case "crt":
      {
        Duration delay = channel.coldRestart(association).toCompletableFuture().get();
        System.out.println("Restart delay: " + delay);
        break;
      }
      case "wrt":
      {
        Duration delay = channel.warmRestart(association).toCompletableFuture().get();
        System.out.println("Restart delay: " + delay);
        break;
      }
      case "rd":
      {
        // ANCHOR: read_directory
        List<FileInfo> items = channel
                .readDirectory(association, ".", DirReadConfig.defaults())
                .toCompletableFuture().get();
        for(FileInfo info : items) {
          printFileInfo(info);
        }
        // ANCHOR_END: read_directory
        break;
      }
      case "gfi":
      {
        // ANCHOR: get_file_info
        FileInfo info = channel.getFileInfo(association, ".").toCompletableFuture().get();
        printFileInfo(info);
        // ANCHOR_END: get_file_info
        break;
      }
      case "rf":
      {
        // ANCHOR: read_file
        channel.readFile(association, ".", FileReadConfig.defaults(), new LoggingFileReader());
        // ANCHOR_END: read_file
        break;
      }
      case "wf":
      {
        // ANCHOR: write_file
        OpenFile file = channel.openFile(association, "hello_world.txt", uint(0), Permissions.none(), uint(0xFFFFFFFF), FileMode.WRITE, ushort(1024)).toCompletableFuture().get();
        channel.writeFileBlock(association, file.fileHandle, uint(0), false, getFileLine()).toCompletableFuture().get();
        channel.writeFileBlock(association, file.fileHandle, uint(1), true, getFileLine()).toCompletableFuture().get();
        channel.closeFile(association, file.fileHandle).toCompletableFuture().get();
        // ANCHOR_END: write_file
      }
      case "lsr":
      {
        channel.checkLinkStatus(association).toCompletableFuture().get();
        break;
      }
      default:
        System.out.println("Unknown command");
        break;
    }
  }

  private static List<UByte> getFileLine() {
    List<UByte> bytes = new ArrayList<>();
    byte[] arr = "hello world\n".getBytes(StandardCharsets.UTF_8);
    for(byte b: arr) {
      bytes.add(ubyte(b));
    }
    return bytes;
  }

  private static void runAssociation(MasterChannel channel, AssociationId association) {
    // Create a periodic poll
    // ANCHOR: add_poll
    PollId poll =
            channel.addPoll(
                    association, Request.classRequest(false, true, true, true), Duration.ofSeconds(5));
    // ANCHOR_END: add_poll

    // start communications
    channel.enable();

    // Handle user input
    BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
    while (true) {
      try {
        final String command = reader.readLine();
        if(command.equals("x")) {
          System.out.println("exiting");
          return;
        }
        runOneCommand(channel, association, poll, command);
      } catch (Exception ex) {
        System.out.println("Error: " + ex);
      }
    }
  }

  private static void runChannel(MasterChannel channel) {

    // Create the association
    // ANCHOR: association_create
    AssociationId association =
        channel.addAssociation(
            ushort(1024),
            getAssociationConfig(),
            new TestReadHandler(),
            new TestAssociationHandler(),
            new TestAssociationInformation());
    // ANCHOR_END: association_create

    runAssociation(channel, association);
  }

  private static void printFileInfo(FileInfo info) {
    System.out.println("file name: " + info.fileName);
    System.out.println("     type: " + info.fileType);
    System.out.println("     size: " + info.size);
    System.out.println("     created: " + info.timeCreated.toString());
  }
}
