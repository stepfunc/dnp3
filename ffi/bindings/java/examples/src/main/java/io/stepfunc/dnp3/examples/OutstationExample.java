// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package io.stepfunc.dnp3.examples;

import static org.joou.Unsigned.*;

import io.stepfunc.dnp3.*;
import io.stepfunc.dnp3.Runtime;
import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import org.joou.UByte;
import org.joou.UInteger;
import org.joou.ULong;
import org.joou.UShort;

public class OutstationExample {

  // number of points of each type in the example's database
  static final int POINT_COUNT = 10;

  private static final int OUTSTATION_ADDRESS = 1024;
  private static final int MASTER_ADDRESS = 1;
  private static final String TCP_ENDPOINT = "127.0.0.1:20000";
  private static final String TLS_ENDPOINT = "127.0.0.1:20001";
  private static final String UDP_LOCAL_ENDPOINT = "127.0.0.1:20000";
  private static final String UDP_REMOTE_ENDPOINT = "127.0.0.1:20001";
  private static final String SERIAL_PORT = "/dev/ttyS0";
  private static final String CA_CERT = "./certs/ca_chain/ca_cert.pem";
  private static final String CA_ENTITY_CERT = "./certs/ca_chain/entity2_cert.pem";
  private static final String CA_ENTITY_KEY = "./certs/ca_chain/entity2_key.pem";
  private static final String SELF_SIGNED_PEER_CERT = "./certs/self_signed/entity1_cert.pem";
  private static final String SELF_SIGNED_ENTITY_CERT = "./certs/self_signed/entity2_cert.pem";
  private static final String SELF_SIGNED_ENTITY_KEY = "./certs/self_signed/entity2_key.pem";

  private static final List<String> TRANSPORTS =
      Arrays.asList("tcp-server", "tcp-client", "udp", "serial", "tls-ca", "tls-self-signed");

  public static void main(String[] args) throws IOException {
    if (args.length < 1 || args.length > 2 || !TRANSPORTS.contains(args[0])) {
      System.err.println(
          "Usage: outstation-example <tcp-server|tcp-client|udp|serial|tls-ca|tls-self-signed> [endpoint|serial-port]");
      System.exit(1);
    }

    // Setup logging
    Logging.configure(new LoggingConfig(), new TestLogger());

    // Create the Tokio runtime
    Runtime runtime = new Runtime(new RuntimeConfig());

    try {
      run(runtime, args);
    } finally {
      // shutting down the runtime also shuts down the outstation
      runtime.shutdown();
    }
  }

  private static void run(Runtime runtime, String[] args) throws IOException {
    final String transport = args[0];
    if (transport.equals("tcp-server")) {
      runTcpServer(runtime, argumentOr(args, TCP_ENDPOINT));
    } else if (transport.equals("tcp-client")) {
      runTcpClient(runtime, argumentOr(args, TCP_ENDPOINT));
    } else if (transport.equals("udp")) {
      runUdp(runtime, argumentOr(args, UDP_REMOTE_ENDPOINT));
    } else if (transport.equals("serial")) {
      runSerial(runtime, argumentOr(args, SERIAL_PORT));
    } else if (transport.equals("tls-ca")) {
      System.out.println("Starting TLS server with CA chain on " + argumentOr(args, TLS_ENDPOINT));
      runTlsServer(runtime, argumentOr(args, TLS_ENDPOINT), getCaTlsConfig());
    } else {
      System.out.println("Starting TLS server with self-signed certificates on " + argumentOr(args, TLS_ENDPOINT));
      runTlsServer(runtime, argumentOr(args, TLS_ENDPOINT), getSelfSignedTlsConfig());
    }
  }

  private static void runTcpServer(Runtime runtime, String endpoint) throws IOException {
    System.out.println("Starting TCP server on " + endpoint);
    // ANCHOR: create_tcp_server
    OutstationServer server = OutstationServer.createTcpServer(runtime, LinkErrorMode.CLOSE, endpoint);
    // ANCHOR_END: create_tcp_server

    try {
      runServer(server);
    } finally {
      server.shutdown();
    }
  }

  private static void runTcpClient(Runtime runtime, String endpoint) throws IOException {
    System.out.println("Starting TCP client to " + endpoint);
    Outstation outstation = Outstation.createTcpClient(
            runtime,
            LinkErrorMode.CLOSE,
            new EndpointList(endpoint),
            new ConnectStrategy(),
            new ConnectOptions(),
            getOutstationConfig(),
            new ExampleOutstationApplication(),
            new ExampleOutstationInformation(),
            new ExampleControlHandler(),
            state -> System.out.println("client state: " + state)
    );

    outstation.transaction(db -> initializeDatabase(db));
    // a TCP client outstation starts disabled: enable it to start connecting
    outstation.enable();
    runOutstation(outstation);
  }

  private static void runUdp(Runtime runtime, String remoteEndpoint) throws IOException {
    System.out.println("Starting UDP transport from " + UDP_LOCAL_ENDPOINT + " to " + remoteEndpoint);
    // ANCHOR: create_udp
    OutstationUdpConfig udpConfig = new OutstationUdpConfig(UDP_LOCAL_ENDPOINT, remoteEndpoint);

    Outstation outstation = Outstation.createUdp(
            runtime,
            udpConfig,
            getOutstationConfig(),
            new ExampleOutstationApplication(),
            new ExampleOutstationInformation(),
            new ExampleControlHandler()
    );
    // ANCHOR_END: create_udp

    outstation.transaction(db -> initializeDatabase(db));
    runOutstation(outstation);
  }

  private static void runSerial(Runtime runtime, String port) throws IOException {
    System.out.println("Starting serial transport on " + port);
    // ANCHOR: create_serial_server
    Outstation outstation = Outstation.createSerialSession2(
            runtime,
            port,
            new SerialSettings(), // 9600 baud, 8 data bits, 1 stop bit, no parity, no flow control
            Duration.ofSeconds(5), // try to open the port every 5 seconds
            getOutstationConfig(),
            new ExampleOutstationApplication(),
            new ExampleOutstationInformation(),
            new ExampleControlHandler(),
            state -> System.out.println("port state: " + state)
    );
    // ANCHOR_END: create_serial_server

    outstation.transaction(db -> initializeDatabase(db));
    runOutstation(outstation);
  }

  private static void runTlsServer(Runtime runtime, String endpoint, TlsServerConfig config) throws IOException {
    // ANCHOR: create_tls_server
    OutstationServer server = OutstationServer.createTlsServer(runtime, LinkErrorMode.CLOSE, endpoint, config);
    // ANCHOR_END: create_tls_server

    try {
      runServer(server);
    } finally {
      server.shutdown();
    }
  }

  private static TlsServerConfig getCaTlsConfig() {
    // ANCHOR: tls_ca_chain_config
    TlsServerConfig config =
            new TlsServerConfig(
                    "test.com",
                    CA_CERT,
                    CA_ENTITY_CERT,
                    CA_ENTITY_KEY,
                    "" // no password
            );
    // ANCHOR_END: tls_ca_chain_config
    return config;
  }

  private static TlsServerConfig getSelfSignedTlsConfig() {
    // ANCHOR: tls_self_signed_config
    TlsServerConfig config =
            new TlsServerConfig(
                    "test.com",
                    SELF_SIGNED_PEER_CERT,
                    SELF_SIGNED_ENTITY_CERT,
                    SELF_SIGNED_ENTITY_KEY,
                    "" // no password
            ).withCertificateMode(CertificateMode.SELF_SIGNED);
    // ANCHOR_END: tls_self_signed_config
    return config;
  }

  private static void runServer(OutstationServer server) throws IOException {
    // ANCHOR: tcp_server_add_outstation
    final Outstation outstation =
            server.addOutstation(
                    getOutstationConfig(),
                    new ExampleOutstationApplication(),
                    new ExampleOutstationInformation(),
                    new ExampleControlHandler(),
                    state -> System.out.println("connection state: " + state),
                    AddressFilter.any());
    // ANCHOR_END: tcp_server_add_outstation

    // Setup initial points before the server starts accepting connections
    // ANCHOR: database_init
    outstation.transaction(db -> initializeDatabase(db));
    // ANCHOR_END: database_init

    // ANCHOR: tcp_server_bind
    server.bind();
    // ANCHOR_END: tcp_server_bind

    runOutstation(outstation);
  }

  private static OutstationConfig getOutstationConfig() {
    // ANCHOR: outstation_config
    // create an outstation configuration with default values
    OutstationConfig config =
        new OutstationConfig(
            // outstation address
            ushort(OUTSTATION_ADDRESS),
            // master address
            ushort(MASTER_ADDRESS),
            // event buffer sizes
            getEventBufferConfig())
    .withDecodeLevel(new DecodeLevel().withApplication(AppDecodeLevel.OBJECT_VALUES));
    // ANCHOR_END: outstation_config
    return config;
  }

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

  // ANCHOR: database_init_transaction
  private static void initializeDatabase(Database db) {
    // add POINT_COUNT points of each type
    for (int i = 0; i < POINT_COUNT; i++) {
      // you can explicitly specify the configuration for each point ...
      db.addBinaryInput(ushort(i), EventClass.CLASS1, new BinaryInputConfig(StaticBinaryInputVariation.GROUP1_VAR1, EventBinaryInputVariation.GROUP2_VAR2));
      // ... or just use the defaults
      db.addDoubleBitBinaryInput(ushort(i), EventClass.CLASS1, new DoubleBitBinaryInputConfig());
      db.addBinaryOutputStatus(ushort(i), EventClass.CLASS1, new BinaryOutputStatusConfig());
      db.addCounter(ushort(i), EventClass.CLASS1, new CounterConfig());
      db.addFrozenCounter(ushort(i), EventClass.CLASS1, new FrozenCounterConfig());
      db.addAnalogInput(ushort(i), EventClass.CLASS1, new AnalogInputConfig(StaticAnalogInputVariation.GROUP30_VAR1, EventAnalogInputVariation.GROUP32_VAR1, 0.0));
      db.addAnalogOutputStatus(ushort(i), EventClass.CLASS1, new AnalogOutputStatusConfig());
      db.addOctetString(ushort(i), EventClass.CLASS1);
    }

    // define device attributes made available to the master
    db.defineStringAttr(ubyte(0), false, AttributeVariations.DEVICE_MANUFACTURERS_NAME, "Step Function I/O");
    db.defineStringAttr(ubyte(0), true, AttributeVariations.USER_ASSIGNED_LOCATION, "Bend, OR");
  }
  // ANCHOR_END: database_init_transaction

  // run the same logic regardless of the transport type
  private static void runOutstation(Outstation outstation) throws IOException {
    boolean binaryValue = false;
    DoubleBit doubleBitBinaryValue = DoubleBit.DETERMINED_OFF;
    boolean binaryOutputStatusValue = false;
    double analogValue = 0.0;
    double analogOutputStatusValue = 0.0;

    final UShort index = ushort(7);
    final Flags onlineFlags = new Flags(Flag.ONLINE);
    final UpdateOptions detectEvent = UpdateOptions.detectEvent();

    final BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));

    printHelp();

    // exit on "x" or at the end of input
    while (true) {
      final String line = reader.readLine();
      if (line == null || line.equals("x")) {
        return;
      } else if (line.equals("help")) {
        printHelp();
      } else if (line.equals("enable")) {
        outstation.enable();
      } else if (line.equals("disable")) {
        outstation.disable();
      } else if (line.equals("bi")) {
        binaryValue = !binaryValue;
        final BinaryInput value = new BinaryInput(index, binaryValue, onlineFlags, now());
        outstation.transaction(db -> db.updateBinaryInput(value, detectEvent));
      } else if (line.equals("dbbi")) {
        doubleBitBinaryValue =
            doubleBitBinaryValue == DoubleBit.DETERMINED_OFF
                ? DoubleBit.DETERMINED_ON
                : DoubleBit.DETERMINED_OFF;
        final DoubleBitBinaryInput value = new DoubleBitBinaryInput(index, doubleBitBinaryValue, onlineFlags, now());
        outstation.transaction(db -> db.updateDoubleBitBinaryInput(value, detectEvent));
      } else if (line.equals("bos")) {
        binaryOutputStatusValue = !binaryOutputStatusValue;
        final BinaryOutputStatus value = new BinaryOutputStatus(index, binaryOutputStatusValue, onlineFlags, now());
        outstation.transaction(db -> db.updateBinaryOutputStatus(value, detectEvent));
      } else if (line.equals("co")) {
        outstation.transaction(db -> {
          // read the counter, which a freeze may have cleared
          final Counter current = db.getCounter(index);
          db.updateCounter(new Counter(index, increment(current.value), onlineFlags, now()), detectEvent);
        });
      } else if (line.equals("fco")) {
        outstation.transaction(db -> {
          // read the frozen counter, which a freeze may have changed
          final FrozenCounter current = db.getFrozenCounter(index);
          db.updateFrozenCounter(new FrozenCounter(index, increment(current.value), onlineFlags, now()), detectEvent);
        });
      } else if (line.equals("ai")) {
        analogValue += 1;
        final AnalogInput value = new AnalogInput(index, analogValue, onlineFlags, now());
        outstation.transaction(db -> db.updateAnalogInput(value, detectEvent));
      } else if (line.equals("aif")) {
        outstation.transaction(db -> db.updateFlags(index, UpdateFlagsType.ANALOG_INPUT, new Flags(Flag.COMM_LOST), now(), detectEvent));
      } else if (line.equals("aos")) {
        analogOutputStatusValue += 1;
        final AnalogOutputStatus value = new AnalogOutputStatus(index, analogOutputStatusValue, onlineFlags, now());
        outstation.transaction(db -> db.updateAnalogOutputStatus(value, detectEvent));
      } else if (line.equals("os")) {
        final List<UByte> value = new ArrayList<>();
        for (byte octet : "Hello".getBytes(StandardCharsets.US_ASCII)) {
          value.add(ubyte(octet));
        }
        outstation.transaction(db -> db.updateOctetString(index, value, detectEvent));
      } else {
        System.out.println("unknown command: " + line);
      }
    }
  }

  private static void printHelp() {
    System.out.println("Commands (each updates the point at index 7):");
    System.out.println("  help - print this list");
    System.out.println("  x - exit");
    System.out.println("  enable - enable the outstation");
    System.out.println("  disable - disable the outstation");
    System.out.println("  bi - toggle binary input");
    System.out.println("  dbbi - toggle double bit binary input");
    System.out.println("  bos - toggle binary output status");
    System.out.println("  co - increment counter");
    System.out.println("  fco - increment frozen counter");
    System.out.println("  ai - increment analog input");
    System.out.println("  aif - set analog input flag to COMM_LOST");
    System.out.println("  aos - increment analog output status");
    System.out.println("  os - set octet string to 'Hello'");
  }

  static Timestamp now() {
    return Timestamp.synchronizedTimestamp(ulong(System.currentTimeMillis()));
  }

  // add one, wrapping to 0 after the largest 32-bit value
  private static UInteger increment(UInteger value) {
    return uint((value.longValue() + 1) & 0xFFFFFFFFL);
  }

  // the optional argument after the transport, or the default
  private static String argumentOr(String[] args, String defaultValue) {
    return args.length > 1 ? args[1] : defaultValue;
  }
}

class ExampleOutstationApplication implements OutstationApplication {

  // ask the master for the time as soon as it connects
  private boolean needTime = true;

  @Override
  public UShort getProcessingDelayMs() {
    return ushort(0);
  }

  // ANCHOR: time_sync
  @Override
  public ApplicationIin getApplicationIin() {
    return new ApplicationIin().withNeedTime(needTime);
  }

  @Override
  public WriteTimeResult writeAbsoluteTime(ULong time) {
    // a real device sets its clock here, and sets needTime again when its clock drifts
    System.out.println("write absolute time: " + time);
    needTime = false;
    return WriteTimeResult.OK;
  }
  // ANCHOR_END: time_sync

  @Override
  public RestartDelay coldRestart() {
    return RestartDelay.notSupported();
  }

  @Override
  public RestartDelay warmRestart() {
    return RestartDelay.notSupported();
  }

  // ANCHOR: freeze
  @Override
  public FreezeResult freezeCountersAll(FreezeType freezeType, DatabaseHandle database) {
    return freezeCounters(0, OutstationExample.POINT_COUNT - 1, freezeType, database);
  }

  @Override
  public FreezeResult freezeCountersRange(
      UShort start, UShort stop, FreezeType freezeType, DatabaseHandle database) {
    return freezeCounters(
        start.intValue(), Math.min(stop.intValue(), OutstationExample.POINT_COUNT - 1), freezeType, database);
  }

  // freezing at a time and interval isn't supported: the default implementations reject it

  private FreezeResult freezeCounters(int start, int stop, FreezeType freezeType, DatabaseHandle database) {
    final boolean clear = freezeType == FreezeType.FREEZE_AND_CLEAR;
    System.out.println("freeze counters " + start + " to " + stop + " (clear: " + clear + ")");
    database.transaction(db -> {
      for (int index = start; index <= stop; index++) {
        // an exception must not escape the transaction, so skip a point that doesn't exist (yet)
        try {
          final Counter counter = db.getCounter(ushort(index));
          final Timestamp time = OutstationExample.now();
          db.updateFrozenCounter(new FrozenCounter(counter.index, counter.value, counter.flags, time), UpdateOptions.detectEvent());
          if (clear) {
            db.updateCounter(new Counter(counter.index, uint(0), counter.flags, time), UpdateOptions.detectEvent());
          }
        } catch (ParamException ex) {
          // no counter at this index
        }
      }
    });
    return FreezeResult.OK;
  }
  // ANCHOR_END: freeze

  // ANCHOR: save_changes
  @Override
  public boolean supportWriteAnalogDeadBands() {
    return true;
  }

  @Override
  public void beginWriteAnalogDeadBands() {
    System.out.println("begin writing analog dead-bands");
  }

  @Override
  public void writeAnalogDeadBand(UShort index, double deadBand) {
    System.out.println("change analog dead-band " + index + " to " + deadBand);
  }

  @Override
  public void endWriteAnalogDeadBands() {
    System.out.println("end writing analog dead-bands: save them here");
  }

  @Override
  public void beginAssignClass() {
    System.out.println("begin assigning classes");
  }

  @Override
  public void assignClassAll(PointType pointType, EventClass pointClass) {
    System.out.println("assign class " + pointClass + " to all " + pointType + " points");
  }

  @Override
  public void assignClassRange(PointType pointType, UShort start, UShort stop, EventClass pointClass) {
    System.out.println("assign class " + pointClass + " to " + pointType + " points " + start + " to " + stop);
  }

  @Override
  public void endAssignClass() {
    System.out.println("end assigning classes: save them here");
  }
  // ANCHOR_END: save_changes

  @Override
  public boolean writeStringAttr(UByte set, UByte variation, StringAttr attrType, String value) {
    System.out.println("write device attribute: set=" + set + " variation=" + variation + " value=" + value);
    // allow writing any string attribute that has been defined as writable
    return true;
  }
}

// ANCHOR: control_handler
class ExampleControlHandler implements ControlHandler {

  @Override
  public void beginFragment() {}

  @Override
  public void endFragment(DatabaseHandle database) {}

  @Override
  public CommandStatus selectG12v1(Group12Var1 control, UShort index, DatabaseHandle database) {
    if (index.intValue() < OutstationExample.POINT_COUNT && (control.code.opType == OpType.LATCH_ON || control.code.opType == OpType.LATCH_OFF)) {
      return CommandStatus.SUCCESS;
    } else {
      return CommandStatus.NOT_SUPPORTED;
    }
  }

  @Override
  public CommandStatus operateG12v1(Group12Var1 control, UShort index, OperateType opType, DatabaseHandle database) {
    System.out.println("operate " + control.code.opType + " on binary output " + index + " via " + opType);
    if (index.intValue() < OutstationExample.POINT_COUNT && (control.code.opType == OpType.LATCH_ON || control.code.opType == OpType.LATCH_OFF)) {
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
    return index.intValue() < OutstationExample.POINT_COUNT ? CommandStatus.SUCCESS : CommandStatus.NOT_SUPPORTED;
  }

  private CommandStatus operateAnalogOutput(double value, UShort index, DatabaseHandle database) {
    if (index.intValue() < OutstationExample.POINT_COUNT) {
      database.transaction(db -> db.updateAnalogOutputStatus(new AnalogOutputStatus(index, value, new Flags(Flag.ONLINE), OutstationExample.now()), UpdateOptions.detectEvent()));
      return CommandStatus.SUCCESS;
    } else {
      return CommandStatus.NOT_SUPPORTED;
    }
  }
}
// ANCHOR_END: control_handler

class ExampleOutstationInformation implements OutstationInformation {

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

class TestLogger implements Logger {

  @Override
  public void onMessage(LogLevel level, String message) {
    System.out.print(message);
  }
}
