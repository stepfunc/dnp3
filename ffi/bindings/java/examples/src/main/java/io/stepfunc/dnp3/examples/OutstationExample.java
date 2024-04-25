package io.stepfunc.dnp3.examples;

import static org.joou.Unsigned.*;

import io.stepfunc.dnp3.*;
import io.stepfunc.dnp3.Runtime;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
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

class TestOutstationApplication implements OutstationApplication {

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
    return RestartDelay.seconds(ushort(60));
  }

  @Override
  public RestartDelay warmRestart() {
    return RestartDelay.notSupported();
  }

  @Override
  public FreezeResult freezeCountersAll(FreezeType freezeType, DatabaseHandle database) {
    return FreezeResult.NOT_SUPPORTED;
  }

  @Override
  public FreezeResult freezeCountersRange(
      UShort start, UShort stop, FreezeType freezeType, DatabaseHandle database) {
    return FreezeResult.NOT_SUPPORTED;
  }

  @Override
  public boolean writeStringAttr(UByte set, UByte variation, StringAttr attrType, String value) {
    // Allow writing any string attributes that have been defined as writable
    return true;
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

// ANCHOR: control_handler
class TestControlHandler implements ControlHandler {

  @Override
  public void beginFragment() {}

  @Override
  public void endFragment(DatabaseHandle database) {}

  @Override
  public CommandStatus selectG12v1(Group12Var1 control, UShort index, DatabaseHandle database) {
    if (index.compareTo(ushort(10)) < 0 && (control.code.opType == OpType.LATCH_ON || control.code.opType == OpType.LATCH_OFF)) {
      return CommandStatus.SUCCESS;
    } else {
      return CommandStatus.NOT_SUPPORTED;
    }
  }

  @Override
  public CommandStatus operateG12v1(Group12Var1 control, UShort index, OperateType opType, DatabaseHandle database) {
    if (index.compareTo(ushort(10)) < 0 && (control.code.opType == OpType.LATCH_ON || control.code.opType == OpType.LATCH_OFF)) {
      boolean status = control.code.opType == OpType.LATCH_ON;
      database.transaction(db -> db.updateBinaryOutputStatus(new BinaryOutputStatus(index, status, new Flags(Flag.ONLINE), OutstationExample.now()), UpdateOptions.detectEvent()));
      return CommandStatus.SUCCESS;
    } else {
      return CommandStatus.NOT_SUPPORTED;
    }
  }

  @Override
  public CommandStatus selectG41v1(int value, UShort index, DatabaseHandle database) {
    return selectAnalogOutput(index);
  }

  @Override
  public CommandStatus operateG41v1(
      int value, UShort index, OperateType opType, DatabaseHandle database) {
    return operateAnalogOutput(value, index, database);
  }

  @Override
  public CommandStatus selectG41v2(short value, UShort index, DatabaseHandle database) {
    return selectAnalogOutput(index);
  }

  @Override
  public CommandStatus operateG41v2(
      short value, UShort index, OperateType opType, DatabaseHandle database) {
    return operateAnalogOutput(value, index, database);
  }

  @Override
  public CommandStatus selectG41v3(float value, UShort index, DatabaseHandle database) {
    return selectAnalogOutput(index);
  }

  @Override
  public CommandStatus operateG41v3(
      float value, UShort index, OperateType opType, DatabaseHandle database) {
    return operateAnalogOutput(value, index, database);
  }

  @Override
  public CommandStatus selectG41v4(double value, UShort index, DatabaseHandle database) {
    return selectAnalogOutput(index);
  }

  @Override
  public CommandStatus operateG41v4(
      double value, UShort index, OperateType opType, DatabaseHandle database) {
    return operateAnalogOutput(value, index, database);
  }

  private CommandStatus selectAnalogOutput(UShort index) {
    return index.compareTo(ushort(10)) < 0 ? CommandStatus.SUCCESS : CommandStatus.NOT_SUPPORTED;
  }

  private CommandStatus operateAnalogOutput(double value, UShort index, DatabaseHandle database) {
    if (index.compareTo(ushort(10)) < 0) {
      database.transaction(db -> db.updateAnalogOutputStatus(new AnalogOutputStatus(index, value, new Flags(Flag.ONLINE), OutstationExample.now()), UpdateOptions.detectEvent()));

      return CommandStatus.SUCCESS;
    }
    else
    {
      return CommandStatus.NOT_SUPPORTED;
    }
  }
}
// ANCHOR_END: control_handler

class TestConnectionStateListener implements ConnectionStateListener {
  @Override
  public void onChange(ConnectionState state) {
    System.out.println("Connection state change: " + state);
  }
}
public class OutstationExample {

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

  static Timestamp now() {
    return Timestamp.synchronizedTimestamp(ulong(System.currentTimeMillis()));
  }

  private static OutstationConfig getOutstationConfig() {
    // ANCHOR: outstation_config
    // create an outstation configuration with default values
    OutstationConfig config =
        new OutstationConfig(
            // outstation address
            ushort(1024),
            // master address
            ushort(1),
            // event buffer sizes
            getEventBufferConfig())
    .withDecodeLevel(new DecodeLevel().withApplication(AppDecodeLevel.OBJECT_VALUES));
    // ANCHOR_END: outstation_config
    return config;
  }

  private static void run(Runtime runtime, String[] args) {

    if(args.length != 1) {
      System.err.println("You must specify a transport");
      System.err.println("Usage: outstation-example <transport> (tcp, serial, tls-ca, tls-self-signed)");
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
        runTls(runtime, getCaTlsConfig());
        break;
      case "tls-self-signed":
        runTls(runtime, getSelfSignedTlsConfig());
        break;
      default:
        System.err.printf("Unknown transport: %s%n", type);
    }
  }

  private static void runTcp(Runtime runtime) {
    // ANCHOR: create_tcp_server
    OutstationServer server = OutstationServer.createTcpServer(runtime, LinkErrorMode.CLOSE, "127.0.0.1:20000");
    // ANCHOR_END: create_tcp_server

    try {
      runServer(server);
    } finally {
      server.shutdown();
    }
  }

  private static void runTls(Runtime runtime, TlsServerConfig config) {
    // ANCHOR: create_tls_server
    OutstationServer server = OutstationServer.createTlsServer(runtime, LinkErrorMode.CLOSE, "127.0.0.1:20001", config);
    // ANCHOR_END: create_tls_server

    try {
      runServer(server);
    } finally {
      server.shutdown();
    }
  }

  private static void runUdp(Runtime runtime) {
    // ANCHOR: create_udp
    OutstationUdpConfig udpConfig = new OutstationUdpConfig(
      "127.0.0.1:20000",
      "127.0.0.1:20001"
    );

    Outstation outstation = Outstation.createUdp(
            runtime,
            udpConfig,
            getOutstationConfig(),
            new TestOutstationApplication(),
            new TestOutstationInformation(),
            new TestControlHandler()
    );
    // ANCHOR_END: create_udp

    runOutstation(outstation);
  }

  private static void runSerial(Runtime runtime) {
    // ANCHOR: create_serial_server
    Outstation outstation = Outstation.createSerialSession2(
            runtime,
            "/dev/pts/4",
            new SerialSettings(),
            Duration.ofSeconds(5), // try to open the port every 5 seconds
            getOutstationConfig(),
            new TestOutstationApplication(),
            new TestOutstationInformation(),
            new TestControlHandler(),
            state -> System.out.println("Port state change: " + state)
    );
    // ANCHOR_END: create_serial_server

    runOutstation(outstation);
  }

  public static void main(String[] args) {
    // Setup logging
    Logging.configure(new LoggingConfig(), new TestLogger());

    // Create the Tokio runtime
    Runtime runtime = new Runtime(new RuntimeConfig());

    try {
      run(runtime, args);
    } finally {
      runtime.shutdown();
    }
  }

  private static TlsServerConfig getSelfSignedTlsConfig() {
    // ANCHOR: tls_self_signed_config
    TlsServerConfig config =
            new TlsServerConfig(
                    "test.com",
                    "./certs/self_signed/entity1_cert.pem",
                    "./certs/self_signed/entity2_cert.pem",
                    "./certs/self_signed/entity2_key.pem",
                    "" // no password
            ).withCertificateMode(CertificateMode.SELF_SIGNED);
    // ANCHOR_END: tls_self_signed_config
    return config;
  }

  private static TlsServerConfig getCaTlsConfig() {
    // ANCHOR: tls_ca_chain_config
    TlsServerConfig config =
            new TlsServerConfig(
                    "test.com",
                    "./certs/ca_chain/ca_cert.pem",
                    "./certs/ca_chain/entity2_cert.pem",
                    "./certs/ca_chain/entity2_key.pem",
                    "" // no password
            );
    // ANCHOR_END: tls_ca_chain_config
    return config;
  }

  // ANCHOR: database_init_function
  private static void initializeDatabase(Database db) {
    // add 10 points of each type
    for (int i = 0; i < 10; i++) {
      // you can explicitly specify the configuration for each point ...
      db.addBinaryInput(ushort(i), EventClass.CLASS1, new BinaryInputConfig(StaticBinaryInputVariation.GROUP1_VAR1, EventBinaryInputVariation.GROUP2_VAR2));
      // ... or just use the defaults
      db.addDoubleBitBinaryInput(ushort(i), EventClass.CLASS1, new DoubleBitBinaryInputConfig());
      db.addBinaryOutputStatus(ushort(i), EventClass.CLASS1, new BinaryOutputStatusConfig());
      db.addCounter(ushort(i), EventClass.CLASS1, new CounterConfig());
      db.addFrozenCounter(ushort(i), EventClass.CLASS1, new FrozenCounterConfig());
      db.addAnalogInput(ushort(i), EventClass.CLASS1, new AnalogInputConfig());
      db.addAnalogOutputStatus(ushort(i), EventClass.CLASS1, new AnalogOutputStatusConfig());
      db.addOctetString(ushort(i), EventClass.CLASS1);
    }

    // define device attributes made available to the master
    db.defineStringAttr(ubyte(0), false, AttributeVariations.DEVICE_MANUFACTURERS_NAME, "Step Function I/O");
    db.defineStringAttr(ubyte(0), true, AttributeVariations.USER_ASSIGNED_LOCATION, "Bend, OR");
  }
  // ANCHOR_END: database_init_function

  private static void runServer(OutstationServer server) {

    // ANCHOR: tcp_server_add_outstation
    final Outstation outstation =
            server.addOutstation(
                    getOutstationConfig(),
                    new TestOutstationApplication(),
                    new TestOutstationInformation(),
                    new TestControlHandler(),
                    new TestConnectionStateListener(),
                    AddressFilter.any());
    // ANCHOR_END: tcp_server_add_outstation

    // ANCHOR: tcp_server_bind
    server.bind();
    // ANCHOR_END: tcp_server_bind

    runOutstation(outstation);
  }

  private static void runOutstation(Outstation outstation) {

    // Setup initial points
    // ANCHOR: database_init
    outstation.transaction((db) -> initializeDatabase(db));
    // ANCHOR_END: database_init

    boolean binaryValue = false;
    DoubleBit doubleBitBinaryValue = DoubleBit.DETERMINED_OFF;
    boolean binaryOutputStatusValue = false;
    long counterValue = 0;
    long frozenCounterValue = 0;
    double analogValue = 0.0;
    double analogOutputStatusValue = 0.0;

    final Flags onlineFlags = new Flags(Flag.ONLINE);
    final UpdateOptions detectEvent = UpdateOptions.detectEvent();

    // Handle user input
    try {
      final BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
      while (true) {
        final String line = reader.readLine();
        switch (line) {
          case "x":
            return;
          case "enable":
            outstation.enable();
            break;
          case "disable":
            outstation.disable();
            break;
          case "bi":
            {
              binaryValue = !binaryValue;
              final boolean pointValue = binaryValue;
              outstation.transaction(
                  db -> {
                    BinaryInput value =
                        new BinaryInput(
                            ushort(7),
                            pointValue,
                            onlineFlags,
                            now());
                    db.updateBinaryInput(value, detectEvent);
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
                    DoubleBitBinaryInput value =
                        new DoubleBitBinaryInput(
                            ushort(7),
                            pointValue,
                            onlineFlags,
                            now());
                    db.updateDoubleBitBinaryInput(value, detectEvent);
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
                            now());
                    db.updateBinaryOutputStatus(value, detectEvent);
                  });
              break;
            }
          case "co":
            {
              counterValue += 1;
              final long pointValue = counterValue;
              outstation.transaction(
                  db -> {
                    Counter value =
                        new Counter(
                            ushort(7),
                            uint(pointValue),
                            onlineFlags,
                            now());
                    db.updateCounter(value, detectEvent);
                  });
              break;
            }
          case "fco":
            {
              frozenCounterValue += 1;
              final long pointValue = frozenCounterValue;
              outstation.transaction(
                  db -> {
                    FrozenCounter value =
                        new FrozenCounter(
                            ushort(7),
                            uint(pointValue),
                            onlineFlags,
                            now());
                    db.updateFrozenCounter(value, detectEvent);
                  });
              break;
            }
          case "ai":
            {
              analogValue += 1;
              final double pointValue = analogValue;
              outstation.transaction(
                  db -> {
                    AnalogInput value =
                        new AnalogInput(
                            ushort(7),
                            pointValue,
                            onlineFlags,
                            now());
                    db.updateAnalogInput(value, detectEvent);
                  });
              break;
            }
          case "aos":
            {
              analogOutputStatusValue += 1;
              final double pointValue = analogOutputStatusValue;
              outstation.transaction(
                  db -> {
                    AnalogOutputStatus value =
                        new AnalogOutputStatus(
                            ushort(7),
                            pointValue,
                            onlineFlags,
                            now());
                    db.updateAnalogOutputStatus(value, detectEvent);
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

                    db.updateOctetString(ushort(7), octetString, detectEvent);
                  });
              break;
            }
          default:
            System.out.printf("Unknown command: %s%n", line);
            break;
        }
      }
    } catch (Exception ex) {
      System.out.println(ex.getMessage());
    }
  }
}
