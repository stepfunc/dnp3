package io.stepfunc.dnp3rs.examples;

import static org.joou.Unsigned.*;

import io.stepfunc.dnp3rs.*;
import io.stepfunc.dnp3rs.Runtime;
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
    System.out.println(
        "Beginning fragment (broadcast: " + header.iin.iin1.isSet(Iin1Flag.BROADCAST) + ")");
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

class TestAssociationHandler implements AssociationHandler {
  @Override
  public TimestampUtc getCurrentTime() {
    return TimestampUtc.valid(ulong(System.currentTimeMillis()));
  }
}

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

  public static void main(String[] args) {
    // ANCHOR: logging_init
    // Initialize logging with the default configuration
    // This may only be called once during program initialization
    Logging.configure(new LoggingConfig(), new ConsoleLogger());
    // ANCHOR_END: logging_init

    // ANCHOR: runtime
    // Create the Tokio runtime
    RuntimeConfig runtimeConfig = new RuntimeConfig();
    runtimeConfig.numCoreThreads = ushort(4);
    try (Runtime runtime = new Runtime(runtimeConfig)) {
      // ANCHOR_END: runtime
      run(runtime);
    } catch (Exception ex) {
      System.out.println(ex.getMessage());
    }
  }

  private static void run(Runtime runtime) throws Exception {

    // Create the master channel
    MasterChannel channel =
        MasterChannel.createTcpChannel(
            runtime,
            LinkErrorMode.CLOSE,
            getMasterChannelConfig(),
            new EndpointList("127.0.0.1:20000"),
            new RetryStrategy(),
            Duration.ofSeconds(1),
            new TestListener());

    // Create the association
    AssociationId association =
        channel.addAssociation(
            ushort(1024),
            getAssociationConfig(),
            new TestReadHandler(),
            new TestAssociationHandler());

    // Create a periodic poll
    PollId poll =
        channel.addPoll(
            association, Request.classRequest(false, true, true, true), Duration.ofSeconds(5));

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
            Commands commands = new Commands();
            G12v1 g12v1 =
                new G12v1(
                    new ControlCode(TripCloseCode.NUL, false, OpType.LATCH_ON),
                    ubyte(1),
                    uint(1000),
                    uint(1000));
            commands.addG12v1u16(ushort(3), g12v1);
            CommandResult result =
                channel
                    .operate(association, CommandMode.SELECT_BEFORE_OPERATE, commands)
                    .toCompletableFuture()
                    .get();
            System.out.println("Result: " + result);
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
