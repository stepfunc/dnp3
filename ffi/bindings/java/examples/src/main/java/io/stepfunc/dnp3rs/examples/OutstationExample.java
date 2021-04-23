package io.stepfunc.dnp3rs.examples;

import static org.joou.Unsigned.*;

import io.stepfunc.dnp3rs.*;
import io.stepfunc.dnp3rs.Runtime;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import org.joou.UByte;
import org.joou.ULong;
import org.joou.UShort;

class TestLogger implements Logger {

  @Override
  public void onMessage(LogLevel level, String message) {
    System.out.print(message);
  }
}

class TestApplication implements OutstationApplication {

  @Override
  public UShort getProcessingDelayMs() {
    return ushort(0);
  }

  @Override
  public WriteTimeResult writeAbsoluteTime(ULong time) {
    return WriteTimeResult.NOT_SUPPORTED;
  }

  @Override
  public ApplicationIin getApplicationIin() {
    return new ApplicationIin();
  }

  @Override
  public RestartDelay coldRestart() {
    return RestartDelay.validSeconds(ushort(60));
  }

  @Override
  public RestartDelay warmRestart() {
    return RestartDelay.notSupported();
  }

  @Override
  public FreezeResult freezeCountersAll(FreezeType freezeType, Database database) {
    return FreezeResult.NOT_SUPPORTED;
  }

  @Override
  public FreezeResult freezeCountersRange(
      UShort start, UShort stop, FreezeType freezeType, Database database) {
    return FreezeResult.NOT_SUPPORTED;
  }
}

class TestOutstationInformation implements OutstationInformation {

  @Override
  public void processRequestFromIdle(RequestHeader header) {}

  @Override
  public void broadcastReceived(FunctionCode functionCode, BroadcastAction action) {}

  @Override
  public void enterSolicitedConfirmWait(UByte ecsn) {}

  @Override
  public void solicitedConfirmTimeout(UByte ecsn) {}

  @Override
  public void solicitedConfirmReceived(UByte ecsn) {}

  @Override
  public void solicitedConfirmWaitNewRequest() {}

  @Override
  public void wrongSolicitedConfirmSeq(UByte ecsn, UByte seq) {}

  @Override
  public void unexpectedConfirm(boolean unsolicited, UByte seq) {}

  @Override
  public void enterUnsolicitedConfirmWait(UByte ecsn) {}

  @Override
  public void unsolicitedConfirmTimeout(UByte ecsn, boolean retry) {}

  @Override
  public void unsolicitedConfirmed(UByte ecsn) {}

  @Override
  public void clearRestartIin() {}
}

class TestControlHandler implements ControlHandler {

  @Override
  public void beginFragment() {}

  @Override
  public void endFragment() {}

  @Override
  public CommandStatus selectG12v1(G12v1 control, UShort index, Database database) {
    return CommandStatus.NOT_SUPPORTED;
  }

  @Override
  public CommandStatus operateG12v1(
      G12v1 control, UShort index, OperateType opType, Database database) {
    return CommandStatus.NOT_SUPPORTED;
  }

  @Override
  public CommandStatus selectG41v1(int control, UShort index, Database database) {
    return CommandStatus.NOT_SUPPORTED;
  }

  @Override
  public CommandStatus operateG41v1(
      int control, UShort index, OperateType opType, Database database) {
    return CommandStatus.NOT_SUPPORTED;
  }

  @Override
  public CommandStatus selectG41v2(short value, UShort index, Database database) {
    return CommandStatus.NOT_SUPPORTED;
  }

  @Override
  public CommandStatus operateG41v2(
      short value, UShort index, OperateType opType, Database database) {
    return CommandStatus.NOT_SUPPORTED;
  }

  @Override
  public CommandStatus selectG41v3(float value, UShort index, Database database) {
    return CommandStatus.NOT_SUPPORTED;
  }

  @Override
  public CommandStatus operateG41v3(
      float value, UShort index, OperateType opType, Database database) {
    return CommandStatus.NOT_SUPPORTED;
  }

  @Override
  public CommandStatus selectG41v4(double value, UShort index, Database database) {
    return CommandStatus.NOT_SUPPORTED;
  }

  @Override
  public CommandStatus operateG41v4(
      double value, UShort index, OperateType opType, Database database) {
    return CommandStatus.NOT_SUPPORTED;
  }
}

class TestConnectionStateListener implements ConnectionStateListener {
  @Override
  public void onChange(ConnectionState state) {
    System.out.println("Connection state change: " + state);
  }
}

public class OutstationExample {

  static OutstationConfig getOutstationConfig() {
    // ANCHOR: outstation_config
    // create an outstation configuration with default values
    OutstationConfig config =
        new OutstationConfig(
            // outstation address
            ushort(1024),
            // master address
            ushort(1));
    // override the default decode log level
    config.decodeLevel.application = AppDecodeLevel.OBJECT_VALUES;
    // ANCHOR_END: outstation_config
    return config;
  }

  public static void main(String[] args) {
    // Setup logging
    Logging.configure(new LoggingConfig(), new TestLogger());

    // Create the Tokio runtime
    try (Runtime runtime = new Runtime(new RuntimeConfig());
        // ANCHOR: create_tcp_server
        TcpServer server = new TcpServer(runtime, LinkErrorMode.CLOSE, "127.0.0.1:20000")
        // ANCHOR_END: create_tcp_server
        ) {
      run(server);
    }
  }

  // ANCHOR: database_init_function
  public static void initializeDatabase(Database db) {
    for (int i = 0; i < 10; i++) {
      db.addBinary(ushort(i), EventClass.CLASS1, new BinaryConfig());
      db.addDoubleBitBinary(ushort(i), EventClass.CLASS1, new DoubleBitBinaryConfig());
      db.addBinaryOutputStatus(ushort(i), EventClass.CLASS1, new BinaryOutputStatusConfig());
      db.addCounter(ushort(i), EventClass.CLASS1, new CounterConfig());
      db.addFrozenCounter(ushort(i), EventClass.CLASS1, new FrozenCounterConfig());
      db.addAnalog(ushort(i), EventClass.CLASS1, new AnalogConfig());
      db.addAnalogOutputStatus(ushort(i), EventClass.CLASS1, new AnalogOutputStatusConfig());
      db.addOctetString(ushort(i), EventClass.CLASS1);
    }
  }
  // ANCHOR_END: database_init_function

  // ANCHOR: event_buffer_config
  private static EventBufferConfig getEventBufferConfig() {
    return new EventBufferConfig(
        ushort(10), // binary
        ushort(10), // double-bit binary
        ushort(10), // binary output status
        ushort(5), // counter
        ushort(5), // frozen counter
        ushort(5), // analog
        ushort(5), // analog output status
        ushort(3) // octet string
        );
  }
  // ANCHOR_END: event_buffer_config

  public static void run(TcpServer server) {

    // ANCHOR: tcp_server_add_outstation
    final Outstation outstation =
        server.addOutstation(
            getOutstationConfig(),
            getEventBufferConfig(),
            new TestApplication(),
            new TestOutstationInformation(),
            new TestControlHandler(),
            new TestConnectionStateListener(),
            AddressFilter.any());
    // ANCHOR_END: tcp_server_add_outstation

    // Setup initial points
    // ANCHOR: database_init
    outstation.transaction((db) -> initializeDatabase(db));
    // ANCHOR_END: database_init

    // Start the outstation
    // ANCHOR: tcp_server_bind
    server.bind();
    // ANCHOR_END: tcp_server_bind

    boolean binaryValue = false;
    DoubleBit doubleBitBinaryValue = DoubleBit.DETERMINED_OFF;
    boolean binaryOutputStatusValue = false;
    long counterValue = 0;
    long frozenCounterValue = 0;
    double analogValue = 0.0;
    double analogOutputStatusValue = 0.0;
    final Flags onlineFlags = new Flags(Flag.ONLINE);

    // Handle user input
    try {
      final BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
      while (true) {
        final String line = reader.readLine();
        switch (line) {
          case "x":
            return;
          case "bi":
            {
              binaryValue = !binaryValue;
              final boolean pointValue = binaryValue;
              outstation.transaction(
                  db -> {
                    Binary value =
                        new Binary(
                            ushort(7),
                            pointValue,
                            onlineFlags,
                            Timestamp.synchronizedTimestamp(ulong(0)));
                    db.updateBinary(value, new UpdateOptions());
                  });
              break;
            }
          case "dbbi":
            {
              doubleBitBinaryValue =
                  doubleBitBinaryValue == DoubleBit.DETERMINED_OFF
                      ? DoubleBit.DETERMINED_ON
                      : DoubleBit.DETERMINED_OFF;
              final DoubleBit pointValue = doubleBitBinaryValue;
              outstation.transaction(
                  db -> {
                    DoubleBitBinary value =
                        new DoubleBitBinary(
                            ushort(7),
                            pointValue,
                            onlineFlags,
                            Timestamp.synchronizedTimestamp(ulong(0)));
                    db.updateDoubleBitBinary(value, new UpdateOptions());
                  });
              break;
            }
          case "bos":
            {
              binaryOutputStatusValue = !binaryOutputStatusValue;
              final boolean pointValue = binaryOutputStatusValue;
              outstation.transaction(
                  db -> {
                    BinaryOutputStatus value =
                        new BinaryOutputStatus(
                            ushort(7),
                            pointValue,
                            onlineFlags,
                            Timestamp.synchronizedTimestamp(ulong(0)));
                    db.updateBinaryOutputStatus(value, new UpdateOptions());
                  });
              break;
            }
          case "co":
            {
              counterValue = ++counterValue;
              final long pointValue = counterValue;
              outstation.transaction(
                  db -> {
                    Counter value =
                        new Counter(
                            ushort(7),
                            uint(pointValue),
                            onlineFlags,
                            Timestamp.synchronizedTimestamp(ulong(0)));
                    db.updateCounter(value, new UpdateOptions());
                  });
              break;
            }
          case "fco":
            {
              frozenCounterValue = ++frozenCounterValue;
              final long pointValue = frozenCounterValue;
              outstation.transaction(
                  db -> {
                    FrozenCounter value =
                        new FrozenCounter(
                            ushort(7),
                            uint(pointValue),
                            onlineFlags,
                            Timestamp.synchronizedTimestamp(ulong(0)));
                    db.updateFrozenCounter(value, new UpdateOptions());
                  });
              break;
            }
          case "ai":
            {
              analogValue = ++analogValue;
              final double pointValue = analogValue;
              outstation.transaction(
                  db -> {
                    Analog value =
                        new Analog(
                            ushort(7),
                            pointValue,
                            onlineFlags,
                            Timestamp.synchronizedTimestamp(ulong(0)));
                    db.updateAnalog(value, new UpdateOptions());
                  });
              break;
            }
          case "aos":
            {
              analogOutputStatusValue = ++analogOutputStatusValue;
              final double pointValue = analogOutputStatusValue;
              outstation.transaction(
                  db -> {
                    AnalogOutputStatus value =
                        new AnalogOutputStatus(
                            ushort(7),
                            pointValue,
                            onlineFlags,
                            Timestamp.synchronizedTimestamp(ulong(0)));
                    db.updateAnalogOutputStatus(value, new UpdateOptions());
                  });
              break;
            }
          case "os":
            {
              outstation.transaction(
                  db -> {
                    List<UByte> octetString = new ArrayList<>();
                    for (byte octet : "Hello".getBytes(StandardCharsets.US_ASCII)) {
                      octetString.add(ubyte(octet));
                    }

                    db.updateOctetString(ushort(7), octetString, new UpdateOptions());
                  });
              break;
            }
          default:
            System.out.println("Unknown command");
            break;
        }
      }
    } catch (Exception ex) {
      System.out.println(ex.getMessage());
    }
  }
}
