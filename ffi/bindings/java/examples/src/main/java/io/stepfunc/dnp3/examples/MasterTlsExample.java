package io.stepfunc.dnp3.examples;

import static org.joou.Unsigned.*;

import io.stepfunc.dnp3.*;
import io.stepfunc.dnp3.Runtime;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.time.Duration;

public class MasterTlsExample {

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

    TlsClientConfig tlsConfig =
        new TlsClientConfig(
            "test.com",
            "./certs/ca_chain/ca_cert.pem",
            "./certs/ca_chain/entity1_cert.pem",
            "./certs/ca_chain/entity1_key.pem");

    // ANCHOR: create_master_channel
    MasterChannel channel =
        MasterChannel.createTlsChannel(
            runtime,
            LinkErrorMode.CLOSE,
            getMasterChannelConfig(),
            new EndpointList("127.0.0.1:20001"),
            tlsConfig,
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
            ushort(10),
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
