using System;
using System.Threading.Tasks;
using System.Collections.Generic;
using dnp3;

class ExampleOutstation
{
    class TestLogger : ILogger
    {
        public void OnMessage(LogLevel level, string message)
        {
            Console.Write($"{message}");
        }
    }

    class TestOutstationApplication : IOutstationApplication
    {
        public ushort GetProcessingDelayMs()
        {
            return 0;
        }

        public WriteTimeResult WriteAbsoluteTime(ulong time)
        {
            return WriteTimeResult.NotSupported;
        }

        public ApplicationIin GetApplicationIin()
        {
            return new ApplicationIin();
        }

        public RestartDelay ColdRestart()
        {
            return RestartDelay.Seconds(60);
        }

        public RestartDelay WarmRestart()
        {
            return RestartDelay.NotSupported();
        }

        public FreezeResult FreezeCountersAll(FreezeType freezeType, Database database) { return FreezeResult.NotSupported; }

        public FreezeResult FreezeCountersRange(ushort start, ushort stop, FreezeType freezeType, Database database) { return FreezeResult.NotSupported; }
    }

    class TestOutstationInformation : IOutstationInformation
    {
        public void ProcessRequestFromIdle(RequestHeader header) { }

        public void BroadcastReceived(FunctionCode functionCode, BroadcastAction action) { }

        public void EnterSolicitedConfirmWait(byte ecsn) { }

        public void SolicitedConfirmTimeout(byte ecsn) { }

        public void SolicitedConfirmReceived(byte ecsn) { }

        public void SolicitedConfirmWaitNewRequest() { }

        public void WrongSolicitedConfirmSeq(byte ecsn, byte seq) { }

        public void UnexpectedConfirm(bool unsolicited, byte seq) { }

        public void EnterUnsolicitedConfirmWait(byte ecsn) { }

        public void UnsolicitedConfirmTimeout(byte ecsn, bool retry) { }

        public void UnsolicitedConfirmed(byte ecsn) { }

        public void ClearRestartIin() { }
    }

    class TestControlHandler : IControlHandler
    {
        public void BeginFragment() { }

        public void EndFragment() { }

        public CommandStatus SelectG12v1(Group12Var1 control, ushort index, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG12v1(Group12Var1 control, ushort index, OperateType opType, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus SelectG41v1(int control, ushort index, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG41v1(int control, ushort index, OperateType opType, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus SelectG41v2(short value, ushort index, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG41v2(short value, ushort index, OperateType opType, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus SelectG41v3(float value, ushort index, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG41v3(float value, ushort index, OperateType opType, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus SelectG41v4(double value, ushort index, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG41v4(double value, ushort index, OperateType opType, Database database) { return CommandStatus.NotSupported; }
    }

    class TestConnectionStateListener : IConnectionStateListener
    {
        public void OnChange(ConnectionState state)
        {
            Console.WriteLine("Connection state change: " + state);
        }
    }

    private static void RunServer(OutstationServer server)
    {
        // ANCHOR: tcp_server_add_outstation
        var outstation = server.AddOutstation(
            GetOutstationConfig(),
            new TestOutstationApplication(),
            new TestOutstationInformation(),
            new TestControlHandler(),
            new TestConnectionStateListener(),
            AddressFilter.Any()
        );
        // ANCHOR_END: tcp_server_add_outstation

        // ANCHOR: tcp_server_bind
        server.Bind();
        // ANCHOR_END: tcp_server_bind

        RunOutstation(outstation);
    }

    private static void RunTcp(Runtime runtime)
    {
        // ANCHOR: create_tcp_server
        var server = OutstationServer.CreateTcpServer(runtime, LinkErrorMode.Close, "127.0.0.1:20000");
        // ANCHOR_END: create_tcp_server

        try
        {
            RunServer(server);
        }
        finally
        {
            server.Shutdown();
        }
    }

    private static void RunSerial(Runtime runtime)
    {
        // ANCHOR: create_serial_server
        var outstation = Outstation.CreateSerialSession(
            runtime,
            "COM1",
            new SerialPortSettings(),
            GetOutstationConfig(),
            new TestOutstationApplication(),
            new TestOutstationInformation(),
            new TestControlHandler()
        );
        // ANCHOR_END: create_serial_server

        
        RunOutstation(outstation);
    }

    private static void RunTls(Runtime runtime, TlsServerConfig config)
    {
        // ANCHOR: create_tls_server
        var server = OutstationServer.CreateTlsServer(runtime, LinkErrorMode.Close, "127.0.0.1:20001", config);
        // ANCHOR_END: create_tls_server

        try
        {
            RunServer(server);
        }
        finally
        {
            server.Shutdown();
        }
    }

    private static TlsServerConfig GetCaTlsConfig()
    {
        // ANCHOR: tls_ca_chain_config
        var config = new TlsServerConfig(
           "test.com",
           "./certs/ca_chain/ca_cert.pem",
           "./certs/ca_chain/entity2_cert.pem",
           "./certs/ca_chain/entity2_key.pem",
           "" // no password
       );        
        // ANCHOR_END: tls_ca_chain_config
        return config;
    }

    private static TlsServerConfig GetSelfSignedTlsConfig()
    {
        // ANCHOR: tls_self_signed_config
        var config = new TlsServerConfig(
            "test.com",
            "./certs/self_signed/entity1.pem",
            "./certs/self_signed/entity2_cert.pem",
            "./certs/self_signed/entity2_key.pem",
            "" // no password
        ).WithCertificateMode(CertificateMode.SelfSigned);
        // ANCHOR_END: tls_self_signed_config
        return config;
    }
         
    public static void Main(string[] args)
    {
        // Initialize logging with the default configuration
        Logging.Configure(
            new LoggingConfig(),
            new TestLogger()
        );
        
        var runtime = new Runtime(new RuntimeConfig { NumCoreThreads = 4 });
       
        if(args.Length != 1)
        {
            System.Console.WriteLine("You must specify the transport type");
            System.Console.WriteLine("Usage: outstation-example <transport> (tcp, serial, tls-ca, tls-self-signed)");
            return;
        }

        try
        {
            var type = args[0];
            switch(type) {
                case "tcp":
                    RunTcp(runtime);
                    break;
                case "serial":
                    RunSerial(runtime);
                    break;
                case "tls-ca":
                    RunTls(runtime, GetCaTlsConfig());
                    break;
                case "tls-self-signed":
                    RunTls(runtime, GetSelfSignedTlsConfig());
                    break;
                default:
                    System.Console.WriteLine("Unknown transport: %s", type);
                    break;
            }
        }
        finally
        {
            runtime.Shutdown();
        }
    }

    // ANCHOR: event_buffer_config
    private static EventBufferConfig GetEventBufferConfig()
    {
        return new EventBufferConfig(
            10, // binary
            10, // double-bit binary
            10, // binary output status
            5,  // counter
            5,  // frozen counter
            5,  // analog
            5,  // analog output status
            3   // octet string
        );
    }
    // ANCHOR_END: event_buffer_config

    private static Timestamp Now()
    {
        return Timestamp.SynchronizedTimestamp((ulong)DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
    }

    private static OutstationConfig GetOutstationConfig()
    {
        // ANCHOR: outstation_config
        // create an outstation configuration with default values
        var config = new OutstationConfig(
            // outstation address
            1024,
            // master address
            1,
            // event buffer sizes
            GetEventBufferConfig()
        ).WithDecodeLevel(DecodeLevel.Nothing().WithApplication(AppDecodeLevel.ObjectValues));
        // ANCHOR_END: outstation_config
        return config;
    }

    private static void RunOutstation(Outstation outstation)
    {
        // Setup initial points
        // ANCHOR: database_init
        outstation.Transaction(db =>
        {
            for (ushort i = 0; i < 10; i++)
            {
                // add points with default values
                db.AddBinaryInput(i, EventClass.Class1, new BinaryInputConfig());
                db.AddDoubleBitBinaryInput(i, EventClass.Class1, new DoubleBitBinaryInputConfig());
                db.AddBinaryOutputStatus(i, EventClass.Class1, new BinaryOutputStatusConfig());
                db.AddCounter(i, EventClass.Class1, new CounterConfig());
                db.AddFrozenCounter(i, EventClass.Class1, new FrozenCounterConfig());
                db.AddAnalogInput(i, EventClass.Class1, new AnalogInputConfig());
                db.AddAnalogOutputStatus(i, EventClass.Class1, new AnalogOutputStatusConfig());
                db.AddOctetString(i, EventClass.Class1);
            }
        });
        // ANCHOR_END: database_init

        var binaryValue = false;
        var doubleBitBinaryValue = DoubleBit.DeterminedOff;
        var binaryOutputStatusValue = false;
        var counterValue = (uint)0;
        var frozenCounterValue = (uint)0;
        var analogValue = 0.0;
        var analogOutputStatusValue = 0.0;

        while (true)
        {
            switch (Console.ReadLine())
            {
                case "x":
                    return;
                case "bi":
                    {
                        outstation.Transaction(db =>
                        {
                            binaryValue = !binaryValue;
                            db.UpdateBinaryInput(new BinaryInput(7, binaryValue, new Flags(Flag.Online), Now()), new UpdateOptions());
                        });
                        break;
                    }
                case "dbbi":
                    {
                        outstation.Transaction(db =>
                        {
                            doubleBitBinaryValue = doubleBitBinaryValue == DoubleBit.DeterminedOff ? DoubleBit.DeterminedOn : DoubleBit.DeterminedOff;
                            db.UpdateDoubleBitBinaryInput(new DoubleBitBinaryInput(7, doubleBitBinaryValue, new Flags(Flag.Online), Now()), new UpdateOptions());
                        });
                        break;
                    }
                case "bos":
                    {
                        outstation.Transaction(db =>
                        {
                            binaryOutputStatusValue = !binaryOutputStatusValue;
                            db.UpdateBinaryOutputStatus(new BinaryOutputStatus(7, binaryOutputStatusValue, new Flags(Flag.Online), Now()), new UpdateOptions());
                        });
                        break;
                    }
                case "co":
                    {
                        outstation.Transaction(db =>
                        {
                            db.UpdateCounter(new Counter(7, ++counterValue, new Flags(Flag.Online), Now()), new UpdateOptions());
                        });
                        break;
                    }
                case "fco":
                    {
                        outstation.Transaction(db =>
                        {
                            db.UpdateFrozenCounter(new FrozenCounter(7, ++frozenCounterValue, new Flags(Flag.Online), Now()), new UpdateOptions());
                        });
                        break;
                    }
                case "ai":
                    {
                        outstation.Transaction(db =>
                        {
                            db.UpdateAnalogInput(new AnalogInput(7, ++analogValue, new Flags(Flag.Online), Now()), new UpdateOptions());
                        });
                        break;
                    }
                case "aos":
                    {
                        outstation.Transaction(db =>
                        {
                            db.UpdateAnalogOutputStatus(new AnalogOutputStatus(7, ++analogOutputStatusValue, new Flags(Flag.Online), Now()), new UpdateOptions());
                        });
                        break;
                    }
                case "os":
                    {
                        outstation.Transaction(db =>
                        {
                            db.UpdateOctetString(7, System.Text.Encoding.ASCII.GetBytes("Hello"), new UpdateOptions());
                        });
                        break;
                    }
                default:
                    Console.WriteLine("Unknown command");
                    break;
            }
        }
    }
}
