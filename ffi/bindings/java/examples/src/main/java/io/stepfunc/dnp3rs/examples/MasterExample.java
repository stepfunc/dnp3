package io.stepfunc.dnp3rs.examples;

import io.stepfunc.dnp3rs.*;
import io.stepfunc.dnp3rs.Runtime;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.time.Duration;
import java.util.List;

import static org.joou.Unsigned.*;

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
  public void beginFragment(ResponseHeader header) {
    System.out.println(
        "Beginning fragment (broadcast: " + header.iin.iin1.isSet(Iin1Flag.BROADCAST) + ")");
  }

  @Override
  public void endFragment(ResponseHeader header) {
    System.out.println("End fragment");
  }

  @Override
  public void handleBinary(HeaderInfo info, List<Binary> it) {
    System.out.println("Binaries:");
    System.out.println("Qualifier: " + info.qualifier);
    System.out.println("Variation: " + info.variation);

    it.forEach(val -> {
      System.out.println("BI " + val.index + ": Value=" + val.value + " Flags=" + val.flags.value
          + " Time=" + val.time.value + " (" + val.time.quality + ")");
    });
  }

  @Override
  public void handleDoubleBitBinary(HeaderInfo info, List<DoubleBitBinary> it) {
    System.out.println("Double Bit Binaries:");
    System.out.println("Qualifier: " + info.qualifier);
    System.out.println("Variation: " + info.variation);

    it.forEach(val -> {
      System.out.println("DBBI " + val.index + ": Value=" + val.value + " Flags=" + val.flags.value
          + " Time=" + val.time.value + " (" + val.time.quality + ")");
    });
  }

  @Override
  public void handleBinaryOutputStatus(HeaderInfo info, List<BinaryOutputStatus> it) {
    System.out.println("Binary Output Statuses:");
    System.out.println("Qualifier: " + info.qualifier);
    System.out.println("Variation: " + info.variation);

    it.forEach(val -> {
      System.out.println("BOS " + val.index + ": Value=" + val.value + " Flags=" + val.flags.value
          + " Time=" + val.time.value + " (" + val.time.quality + ")");
    });
  }

  @Override
  public void handleCounter(HeaderInfo info, List<Counter> it) {
    System.out.println("Counters:");
    System.out.println("Qualifier: " + info.qualifier);
    System.out.println("Variation: " + info.variation);

    it.forEach(val -> {
      System.out.println("Counter " + val.index + ": Value=" + val.value + " Flags="
          + val.flags.value + " Time=" + val.time.value + " (" + val.time.quality + ")");
    });
  }

  @Override
  public void handleFrozenCounter(HeaderInfo info, List<FrozenCounter> it) {
    System.out.println("Frozen Counters:");
    System.out.println("Qualifier: " + info.qualifier);
    System.out.println("Variation: " + info.variation);

    it.forEach(val -> {
      System.out.println("Frozen Counter " + val.index + ": Value=" + val.value + " Flags="
          + val.flags.value + " Time=" + val.time.value + " (" + val.time.quality + ")");
    });
  }

  @Override
  public void handleAnalog(HeaderInfo info, List<Analog> it) {
    System.out.println("Analogs:");
    System.out.println("Qualifier: " + info.qualifier);
    System.out.println("Variation: " + info.variation);

    it.forEach(val -> {
      System.out.println("AI " + val.index + ": Value=" + val.value + " Flags=" + val.flags.value
          + " Time=" + val.time.value + " (" + val.time.quality + ")");
    });
  }

  @Override
  public void handleAnalogOutputStatus(HeaderInfo info, List<AnalogOutputStatus> it) {
    System.out.println("Analog Output Statuses:");
    System.out.println("Qualifier: " + info.qualifier);
    System.out.println("Variation: " + info.variation);

    it.forEach(val -> {
      System.out.println("AOS " + val.index + ": Value=" + val.value + " Flags=" + val.flags.value
          + " Time=" + val.time.value + " (" + val.time.quality + ")");
    });
  }

  @Override
  public void handleOctetString(HeaderInfo info, List<OctetString> it) {
    System.out.println("Octet String:");
    System.out.println("Qualifier: " + info.qualifier);
    System.out.println("Variation: " + info.variation);

    it.forEach(val -> {
      System.out.print("Octet String " + val.index + ": Value=");
      val.value.forEach(b -> System.out.print(String.format("%02X", b.value.byteValue()) + " "));
      System.out.println();
    });
  }
}


class TestTimeProvider implements TimeProvider {
  @Override
  public TimeProviderTimestamp getTime() {
    return TimeProviderTimestamp.valid(ulong(System.currentTimeMillis()));
  }
}


public class MasterExample {

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
    // Create the master
    MasterConfig masterConfig = new MasterConfig(ushort(1));
    masterConfig.decodeLevel.application = AppDecodeLevel.OBJECT_VALUES;

    Master master = Master.createTcpSession(runtime, LinkErrorMode.CLOSE, masterConfig,
        new EndpointList("127.0.0.1:20000"), new TestListener());

    // Create the association
    AssociationConfig associationConfig = new AssociationConfig(new EventClasses(true, true, true),
        new EventClasses(true, true, true), Classes.all(), new EventClasses(false, false, false));
    associationConfig.autoTimeSync = AutoTimeSync.LAN;
    associationConfig.keepAliveTimeout = Duration.ofSeconds(60);

    TestReadHandler readHandler = new TestReadHandler();
    AssociationHandlers associationHandlers =
        new AssociationHandlers(readHandler, readHandler, readHandler);
    Association association = master.addAssociation(ushort(1024), associationConfig,
        associationHandlers, new TestTimeProvider());

    // Create a periodic poll
    Request pollRequest = Request.classRequest(false, true, true, true);
    Poll poll = association.addPoll(pollRequest, Duration.ofSeconds(5));

    // Handle user input
    BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
    while (true) {
      String line = reader.readLine();
      switch (line) {
        case "x":
          return;
        case "dln":
          master.setDecodeLevel(new DecodeLevel());
          break;
        case "dlv":
          DecodeLevel level = new DecodeLevel();
          level.application = AppDecodeLevel.OBJECT_VALUES;
          master.setDecodeLevel(level);
          break;
        case "rao": {
          Request request = new Request();
          request.addAllObjectsHeader(Variation.GROUP40_VAR0);
          ReadResult result = association.read(request).toCompletableFuture().get();
          System.out.println("Result: " + result);
          break;
        }
        case "rmo": {
          Request request = new Request();
          request.addAllObjectsHeader(Variation.GROUP10_VAR0);
          request.addAllObjectsHeader(Variation.GROUP40_VAR0);
          ReadResult result = association.read(request).toCompletableFuture().get();
          System.out.println("Result: " + result);
          break;
        }
        case "cmd": {
          Command command = new Command();
          G12v1 g12v1 = new G12v1(new ControlCode(TripCloseCode.NUL, false, OpType.LATCH_ON),
              ubyte(1), uint(1000), uint(1000));
          command.addU16g12v1(ushort(3), g12v1);
          CommandResult result = association.operate(CommandMode.SELECT_BEFORE_OPERATE, command)
              .toCompletableFuture().get();
          System.out.println("Result: " + result);
          break;
        }
        case "evt":
          poll.demand();
          break;
        case "lts": {
          TimeSyncResult result =
              association.performTimeSync(TimeSyncMode.LAN).toCompletableFuture().get();
          System.out.println("Result: " + result);
          break;
        }
        case "nts": {
          TimeSyncResult result =
              association.performTimeSync(TimeSyncMode.NON_LAN).toCompletableFuture().get();
          System.out.println("Result: " + result);
          break;
        }
        case "crt": {
          RestartResult result = association.coldRestart().toCompletableFuture().get();
          System.out.println("Result: " + result);
          break;
        }
        case "wrt": {
          RestartResult result = association.warmRestart().toCompletableFuture().get();
          System.out.println("Result: " + result);
          break;
        }
        case "lsr": {
          LinkStatusResult result = association.checkLinkStatus().toCompletableFuture().get();
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
