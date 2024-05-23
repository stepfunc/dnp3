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

        FreezeResult IOutstationApplication.FreezeCountersAll(FreezeType freezeType, DatabaseHandle database) { return FreezeResult.NotSupported; }

        FreezeResult IOutstationApplication.FreezeCountersRange(ushort start, ushort stop, FreezeType freezeType, DatabaseHandle database) { return FreezeResult.NotSupported; }

        FreezeResult IOutstationApplication.FreezeCountersAllAtTime(DatabaseHandle databaseHandle, ulong time, uint interval)
        {
            return FreezeResult.NotSupported;
        }

        FreezeResult IOutstationApplication.FreezeCountersRangeAtTime(ushort start, ushort stop, DatabaseHandle databaseHandle, ulong time, uint interval)
        {
            return FreezeResult.NotSupported;
        }

        bool IOutstationApplication.SupportWriteAnalogDeadBands()
        {
            return false;
        }

        void IOutstationApplication.BeginWriteAnalogDeadBands() {}

        void IOutstationApplication.WriteAnalogDeadBand(ushort index, double deadBand) {}

        void IOutstationApplication.EndWriteAnalogDeadBands() {}

        bool IOutstationApplication.WriteStringAttr(byte set, byte variation, StringAttr attrType, string value)
        {
            // Allow writing any string attributes that have been defined as writable
            return true;
        }

        bool IOutstationApplication.WriteFloatAttr(byte set, byte variation, FloatAttr attrType, float value)
        {            
            return false;
        }

        bool IOutstationApplication.WriteDoubleAttr(byte set, byte variation, FloatAttr attrType, double value)
        {
            return false;
        }

        bool IOutstationApplication.WriteUintAttr(byte set, byte variation, UintAttr attrType, uint value)
        {
            return false;
        }

        bool IOutstationApplication.WriteIntAttr(byte set, byte variation, IntAttr attrType, int value)
        {
            return false;
        }

        bool IOutstationApplication.WriteOctetStringAttr(byte set, byte variation, OctetStringAttr attrType, ICollection<byte> value)
        {
            return false;
        }

        bool IOutstationApplication.WriteBitStringAttr(byte set, byte variation, BitStringAttr attrType, ICollection<byte> value)
        {
            return false;
        }

        bool IOutstationApplication.WriteTimeAttr(byte set, byte variation, TimeAttr attrType, ulong value)
        {
            return false;
        }

        void IOutstationApplication.BeginConfirm()
        {

        }

        void IOutstationApplication.EventCleared(ulong id)
        {

        }

        void IOutstationApplication.EndConfirm(BufferState state)
        {

        }
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

    // ANCHOR: control_handler
    class TestControlHandler : IControlHandler
    {
        public void BeginFragment() { }

        public void EndFragment(DatabaseHandle database) { }

        public CommandStatus SelectG12v1(Group12Var1 control, ushort index, DatabaseHandle database)
        {
            if (index < 10 && (control.Code.OpType == OpType.LatchOn || control.Code.OpType == OpType.LatchOff))
            {
                return CommandStatus.Success;
            }
            else
            {
                return CommandStatus.NotSupported;
            }
        }

        public CommandStatus OperateG12v1(Group12Var1 control, ushort index, OperateType opType, DatabaseHandle database)
        {
            if (index < 10 && (control.Code.OpType == OpType.LatchOn || control.Code.OpType == OpType.LatchOff))
            {
                var status = (control.Code.OpType == OpType.LatchOn);
                database.Transaction(db =>
                    db.UpdateBinaryOutputStatus(new BinaryOutputStatus(index, status, new Flags(Flag.Online), Now()), UpdateOptions.DetectEvent())
                );
                return CommandStatus.Success;
            }
            else
            {
                return CommandStatus.NotSupported;
            }
        }

        public CommandStatus SelectG41v1(int value, ushort index, DatabaseHandle database)
        {
            return SelectAnalogOutput(index);
        }

        public CommandStatus OperateG41v1(int value, ushort index, OperateType opType, DatabaseHandle database)
        {
            return OperateAnalogOutput(value, index, database);
        }

        public CommandStatus SelectG41v2(short value, ushort index, DatabaseHandle database)
        {
            return SelectAnalogOutput(index);
        }

        public CommandStatus OperateG41v2(short value, ushort index, OperateType opType, DatabaseHandle database)
        {
            return OperateAnalogOutput(value, index, database);
        }

        public CommandStatus SelectG41v3(float value, ushort index, DatabaseHandle database)
        {
            return SelectAnalogOutput(index);
        }

        public CommandStatus OperateG41v3(float value, ushort index, OperateType opType, DatabaseHandle database)
        {
            return OperateAnalogOutput(value, index, database);
        }

        public CommandStatus SelectG41v4(double value, ushort index, DatabaseHandle database)
        {
            return SelectAnalogOutput(index);
        }

        public CommandStatus OperateG41v4(double value, ushort index, OperateType opType, DatabaseHandle database)
        {
            return OperateAnalogOutput(value, index, database);
        }

        private CommandStatus SelectAnalogOutput(ushort index)
        {
            return index < 10 ? CommandStatus.Success : CommandStatus.NotSupported;
        }

        private CommandStatus OperateAnalogOutput(double value, ushort index, DatabaseHandle database)
        {
            if (index < 10)
            {
                database.Transaction(db =>
                    db.UpdateAnalogOutputStatus(new AnalogOutputStatus(index, value, new Flags(Flag.Online), Now()), UpdateOptions.DetectEvent())
                );
                return CommandStatus.Success;
            }
            else
            {
                return CommandStatus.NotSupported;
            }
        }
    }
    // ANCHOR_END: control_handler

    class TestConnectionStateListener : IConnectionStateListener
    {
        public void OnChange(ConnectionState state)
        {
            Console.WriteLine("Connection state change: " + state);
        }
    }

    class TestPortStateListener : IPortStateListener
    {
        public void OnChange(PortState state)
        {
            Console.WriteLine("Port state change: " + state);
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
        var outstation = Outstation.CreateSerialSession2(
            runtime,
            "COM1",
            new SerialSettings(),
            TimeSpan.FromSeconds(5), // try to open the port every 5 seconds
            GetOutstationConfig(),
            new TestOutstationApplication(),
            new TestOutstationInformation(),
            new TestControlHandler(),
            new TestPortStateListener()
        );
        // ANCHOR_END: create_serial_server


        RunOutstation(outstation);
    }

    private static void RunUdp(Runtime runtime)
    {
        // ANCHOR: create_udp
        var config = new OutstationUdpConfig(
            "127.0.0.1:20000",
            "127.0.0.1:20001"
        );

        var outstation = Outstation.CreateUdp(
            runtime,
            config,
            GetOutstationConfig(),
            new TestOutstationApplication(),
            new TestOutstationInformation(),
            new TestControlHandler()
        );
        // ANCHOR_END: create_udp

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

        if (args.Length != 1)
        {
            System.Console.WriteLine("You must specify the transport type");
            System.Console.WriteLine("Usage: outstation-example <transport> (tcp, serial, tls-ca, tls-self-signed)");
            return;
        }

        try
        {
            var type = args[0];
            switch (type)
            {
                case "tcp":
                    RunTcp(runtime);
                    break;
                case "udp":
                    RunUdp(runtime);
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
            // add 10 points of each type
            for (ushort i = 0; i < 10; i++)
            {
                // you can explicitly specify the configuration for each point ...
                db.AddBinaryInput(i, EventClass.Class1,
                    new BinaryInputConfig(StaticBinaryInputVariation.Group1Var1, EventBinaryInputVariation.Group2Var2)
                );
                // ... or just use the defaults
                db.AddDoubleBitBinaryInput(i, EventClass.Class1, new DoubleBitBinaryInputConfig());
                db.AddBinaryOutputStatus(i, EventClass.Class1, new BinaryOutputStatusConfig());
                db.AddCounter(i, EventClass.Class1, new CounterConfig());
                db.AddFrozenCounter(i, EventClass.Class1, new FrozenCounterConfig());
                db.AddAnalogInput(i, EventClass.Class1, new AnalogInputConfig());
                db.AddAnalogOutputStatus(i, EventClass.Class1, new AnalogOutputStatusConfig());
                db.AddOctetString(i, EventClass.Class1);                
            }

            // define device attributes made available to the master
            db.DefineStringAttr(0, false, AttributeVariations.DeviceManufacturersName, "Step Function I/O");
            db.DefineStringAttr(0, true, AttributeVariations.UserAssignedLocation, "Bend, OR");
        });
        // ANCHOR_END: database_init

        var binaryValue = false;
        var doubleBitBinaryValue = DoubleBit.DeterminedOff;
        var binaryOutputStatusValue = false;
        var counterValue = (uint)0;
        var frozenCounterValue = (uint)0;
        var analogValue = 0.0;
        var analogOutputStatusValue = 0.0;

        var onlineFlags = new Flags(Flag.Online);
        var detectEvent = UpdateOptions.DetectEvent();

        while (true)
        {
            switch (Console.ReadLine())
            {
                case "x":
                    return;
                case "enable":
                    {
                        outstation.Enable();
                        break;
                    }
                case "disable":
                    {
                        outstation.Disable();
                        break;
                    }
                case "bi":
                    {
                        outstation.Transaction(db =>
                        {
                            binaryValue = !binaryValue;
                            db.UpdateBinaryInput(new BinaryInput(7, binaryValue, onlineFlags, Now()), detectEvent);
                        });
                        break;
                    }
                case "dbbi":
                    {
                        outstation.Transaction(db =>
                        {
                            doubleBitBinaryValue = doubleBitBinaryValue == DoubleBit.DeterminedOff ? DoubleBit.DeterminedOn : DoubleBit.DeterminedOff;
                            db.UpdateDoubleBitBinaryInput(new DoubleBitBinaryInput(7, doubleBitBinaryValue, onlineFlags, Now()), detectEvent);
                        });
                        break;
                    }
                case "bos":
                    {
                        outstation.Transaction(db =>
                        {
                            binaryOutputStatusValue = !binaryOutputStatusValue;
                            db.UpdateBinaryOutputStatus(new BinaryOutputStatus(7, binaryOutputStatusValue, onlineFlags, Now()), detectEvent);
                        });
                        break;
                    }
                case "co":
                    {
                        outstation.Transaction(db =>
                        {
                            db.UpdateCounter(new Counter(7, ++counterValue, onlineFlags, Now()), detectEvent);
                        });
                        break;
                    }
                case "fco":
                    {
                        outstation.Transaction(db =>
                        {
                            db.UpdateFrozenCounter(new FrozenCounter(7, ++frozenCounterValue, onlineFlags, Now()), detectEvent);
                        });
                        break;
                    }
                case "ai":
                    {
                        outstation.Transaction(db =>
                        {
                            db.UpdateAnalogInput(new AnalogInput(7, ++analogValue, onlineFlags, Now()), detectEvent);
                        });
                        break;
                    }
                case "aif":
                    {
                        outstation.Transaction(db =>
                        {
                            db.UpdateFlags(7, UpdateFlagsType.AnalogInput, new Flags(Flag.CommLost), Now(), detectEvent);
                        });
                        break;
                    }                    
                case "aos":
                    {
                        outstation.Transaction(db =>
                        {
                            db.UpdateAnalogOutputStatus(new AnalogOutputStatus(7, ++analogOutputStatusValue, onlineFlags, Now()), detectEvent);
                        });
                        break;
                    }
                case "os":
                    {
                        outstation.Transaction(db =>
                        {
                            db.UpdateOctetString(7, System.Text.Encoding.ASCII.GetBytes("Hello"), detectEvent);
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
