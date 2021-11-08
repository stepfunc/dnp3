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

class TestListener implements ClientStateListener {

  @Override
  public void onChange(ClientState state) {
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
  public void handleBinary(HeaderInfo info, List<Binary> it) {
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
  public void handleDoubleBitBinary(HeaderInfo info, List<DoubleBitBinary> it) {
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
  public void handleAnalog(HeaderInfo info, List<Analog> it) {
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
              b -> System.out.print(String.format("%02X", b.value.byteValue()) + " "));
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
    AssociationConfig config =
        new AssociationConfig(
            // disable unsolicited first (Class 1/2/3)
            EventClasses.all(),
            // after the integrity poll, enable unsolicited (Class 1/2/3)
            EventClasses.all(),
            // perform startup integrity poll with Class 1/2/3/0
            Classes.all(),
            // don't automatically scan Class 1/2/3 when the corresponding IIN bit is asserted
            EventClasses.none());
    config.autoTimeSync = AutoTimeSync.LAN;
    config.keepAliveTimeout = Duration.ofSeconds(60);
    return config;
  }
  // ANCHOR_END: association_config

  // ANCHOR: runtime_config
  public static RuntimeConfig getRuntimeConfig() {
    RuntimeConfig config = new RuntimeConfig();
    config.numCoreThreads = ushort(4);
    return config;
  }
  // ANCHOR_END: runtime_config

  public static void main(String[] args) throws Exception {
    // Initialize logging with the default configuration
    // This may only be called once during program initialization
    // ANCHOR: logging_init
    Logging.configure(new LoggingConfig(), new ConsoleLogger());
    // ANCHOR_END: logging_init

    // ANCHOR: runtime
    Runtime runtime = new Runtime(getRuntimeConfig());
    // ANCHOR_END: runtime

    // ANCHOR: create_master_channel
    MasterChannel channel =
        MasterChannel.createTcpChannel(
            runtime,
            LinkErrorMode.CLOSE,
            getMasterChannelConfig(),
            new EndpointList("127.0.0.1:20000"),
            new ConnectStrategy(),
            new TestListener());
    // ANCHOR_END: create_master_channel

    try {
      run(channel);
    } finally {
      // ANCHOR: runtime_shutdown
      runtime.shutdown();
      // ANCHOR_END: runtime_shutdown
    }
  }

  private static void run(MasterChannel channel) throws Exception {

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
      String line = reader.readLine();
      switch (line) {
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
            ReadResult result = channel.read(association, request).toCompletableFuture().get();
            System.out.println("Result: " + result);
            break;
          }
        case "rmo":
          {
            Request request = new Request();
            request.addAllObjectsHeader(Variation.GROUP10_VAR0);
            request.addAllObjectsHeader(Variation.GROUP40_VAR0);
            ReadResult result = channel.read(association, request).toCompletableFuture().get();
            System.out.println("Result: " + result);
            break;
          }
        case "cmd":
          {
            // ANCHOR: assoc_control
            Commands commands = new Commands();
            Group12Var1 control =
                new Group12Var1(
                    new ControlCode(TripCloseCode.NUL, false, OpType.LATCH_ON),
                    ubyte(1),
                    uint(1000),
                    uint(1000));
            commands.addG12V1U16(ushort(3), control);

            CommandResult result =
                channel
                    .operate(association, CommandMode.SELECT_BEFORE_OPERATE, commands)
                    .toCompletableFuture()
                    .get();

            System.out.println("Result: " + result);
            // ANCHOR_END: assoc_control
            break;
          }
        case "evt":
          channel.demandPoll(poll);
          break;

        case "lts":
          {
            TimeSyncResult result =
                channel.synchronizeTime(association, TimeSyncMode.LAN).toCompletableFuture().get();
            System.out.println("Result: " + result);
            break;
          }
        case "nts":
          {
            TimeSyncResult result =
                channel
                    .synchronizeTime(association, TimeSyncMode.NON_LAN)
                    .toCompletableFuture()
                    .get();
            System.out.println("Result: " + result);
            break;
          }
        case "crt":
          {
            RestartResult result = channel.coldRestart(association).toCompletableFuture().get();
            System.out.println("Result: " + result);
            break;
          }
        case "wrt":
          {
            RestartResult result = channel.warmRestart(association).toCompletableFuture().get();
            System.out.println("Result: " + result);
            break;
          }
        case "lsr":
          {
            LinkStatusResult result =
                channel.checkLinkStatus(association).toCompletableFuture().get();
            System.out.println("Result: " + result);
            break;
          }
        default:
          System.out.println("Unknown command");
          break;
      }
    }
  }
}
