package io.stepfunc.dnp3.examples;

import static org.joou.Unsigned.*;

import io.stepfunc.dnp3.*;
import io.stepfunc.dnp3.Runtime;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import org.joou.UByte;

public class OutstationTlsExample {

  static OutstationConfig getOutstationConfig() {
    // ANCHOR: outstation_config
    // create an outstation configuration with default values
    OutstationConfig config =
        new OutstationConfig(
            // outstation address
            ushort(10),
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
    Runtime runtime = new Runtime(new RuntimeConfig());

    // ANCHOR: tls_self_signed_config
    TlsServerConfig selfSignedTlsConfig =
        new TlsServerConfig(
            "test.com",
            "./certs/self_signed/entity1_cert.pem",
            "./certs/self_signed/entity2_cert.pem",
            "./certs/self_signed/entity2_key.pem",
            "" // no password
            );
    selfSignedTlsConfig.certificateMode = CertificateMode.SELF_SIGNED_CERTIFICATE;
    // ANCHOR_END: tls_self_signed_config

    // ANCHOR: tls_ca_chain_config
    TlsServerConfig caChainTlsConfig =
        new TlsServerConfig(
            "test.com",
            "./certs/ca_chain/ca_cert.pem",
            "./certs/ca_chain/entity2_cert.pem",
            "./certs/ca_chain/entity2_key.pem",
            "" // no password
            );
    // ANCHOR_END: tls_ca_chain_config

    TlsServerConfig tlsConfig = caChainTlsConfig;

    // ANCHOR: create_tls_server
    TcpServer server =
        TcpServer.createTlsServer(runtime, LinkErrorMode.CLOSE, "127.0.0.1:20001", tlsConfig);
    // ANCHOR_END: create_tls_server

    try {
      run(server);
    } finally {
      runtime.shutdown();
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
