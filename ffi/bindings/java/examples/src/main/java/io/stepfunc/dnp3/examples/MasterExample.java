package io.stepfunc.dnp3.examples;

import static org.joou.Unsigned.*;

import io.stepfunc.dnp3.*;
import io.stepfunc.dnp3.Runtime;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.time.Duration;
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

class TestPortStateListener implements PortStateListener {

  @Override
  public void onChange(PortState state) {
    System.out.println(state);
  }
}

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
    System.out.println("Binaries:");
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
    System.out.println("Double Bit Binaries:");
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
    System.out.println("Analogs:");
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
    System.out.println("Octet String:");
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
}

// ANCHOR: association_handler
class TestAssociationHandler implements AssociationHandler {
  @Override
  public UtcTimestamp getCurrentTime() {
    return UtcTimestamp.valid(ulong(System.currentTimeMillis()));
  }
}
// ANCHOR_END: association_handler

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

  private static void runTls(Runtime runtime, TlsClientConfig config) throws Exception {
    // ANCHOR: create_tls_channel
    MasterChannel channel =
            MasterChannel.createTlsChannel(
                    runtime,
                    LinkErrorMode.CLOSE,
                    getMasterChannelConfig(),
                    new EndpointList("127.0.0.1:20001"),
                    config,
                    new ConnectStrategy(),
                    new TestClientStateListener());
    // ANCHOR_END: create_tls_channel

    try {
      runChannel(channel);
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
                    new SerialPortSettings(),
                    Duration.ofSeconds(5),
                    new TestPortStateListener());
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

  private static void runChannel(MasterChannel channel) throws Exception {

    // Create the association
    // ANCHOR: association_create
    AssociationId association =
        channel.addAssociation(
            ushort(1024),
            getAssociationConfig(),
            new TestReadHandler(),
            new TestAssociationHandler());
    // ANCHOR_END: association_create

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
        switch (reader.readLine()) {
          case "x":
            return;
          case "enable":
            channel.enable();
            break;
          case "disable":
            channel.disable();
            break;
          case "dln":
            channel.setDecodeLevel(new DecodeLevel());
            break;
          case "dlv":
            DecodeLevel level = new DecodeLevel();
            level.application = AppDecodeLevel.OBJECT_VALUES;
            channel.setDecodeLevel(level);
            break;
          case "rao":
          {
            Request request = new Request();
            request.addAllObjectsHeader(Variation.GROUP40_VAR0);
            channel.read(association, request).toCompletableFuture().get();
            System.out.println("read success!");
            break;
          }
          case "rmo":
          {
            Request request = new Request();
            request.addAllObjectsHeader(Variation.GROUP10_VAR0);
            request.addAllObjectsHeader(Variation.GROUP40_VAR0);
            channel.read(association, request).toCompletableFuture().get();
            System.out.println("read success!");
            break;
          }
          case "cmd":
          {
            // ANCHOR: assoc_control
            CommandSet commands = new CommandSet();
            Group12Var1 control =
                    new Group12Var1(
                            new ControlCode(TripCloseCode.NUL, false, OpType.LATCH_ON),
                            ubyte(1),
                            uint(1000),
                            uint(1000));
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
            System.out.println("Time sync success!");
            break;
          }
          case "nts":
          {
            channel.synchronizeTime(association, TimeSyncMode.NON_LAN).toCompletableFuture().get();
            System.out.println("Time sync success!");
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
          case "lsr":
          {
            channel.checkLinkStatus(association).toCompletableFuture().get();
            System.out.println("Link status success!");
            break;
          }
          default:
            System.out.println("Unknown command");
            break;
        }
      } catch (ParamException ex) {
        System.out.println("Error: " + ex);
      }
    }
  }
}
