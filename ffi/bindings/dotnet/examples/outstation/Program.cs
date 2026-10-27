// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

using System;
using System.Collections.Generic;
using dnp3;

class ExampleOutstation
{
    const ushort OutstationAddress = 1024;
    const ushort MasterAddress = 1;
    const string TcpEndpoint = "127.0.0.1:20000";
    const string TlsEndpoint = "127.0.0.1:20001";
    const string UdpLocalEndpoint = "127.0.0.1:20000";
    const string UdpRemoteEndpoint = "127.0.0.1:20001";
    const string SerialPort = "/dev/ttyS0";
    const string TlsDomain = "test.com";
    const string CaCert = "./certs/ca_chain/ca_cert.pem";
    const string CaEntityCert = "./certs/ca_chain/entity2_cert.pem";
    const string CaEntityKey = "./certs/ca_chain/entity2_key.pem";
    const string SelfSignedPeerCert = "./certs/self_signed/entity1_cert.pem";
    const string SelfSignedEntityCert = "./certs/self_signed/entity2_cert.pem";
    const string SelfSignedEntityKey = "./certs/self_signed/entity2_key.pem";
    // number of points of each type in the example's database
    const ushort PointCount = 10;

    public static int Main(string[] args)
    {
        // the transport plus an optional endpoint (TCP/TLS), remote endpoint (UDP) or serial port
        var argument = args.Length == 2 ? args[1] : null;
        Action<Runtime> run = args.Length < 1 || args.Length > 2 ? null : args[0] switch
        {
            "tcp-server" => runtime => RunTcpServer(runtime, argument ?? TcpEndpoint),
            "tcp-client" => runtime => RunTcpClient(runtime, argument ?? TcpEndpoint),
            "udp" => runtime => RunUdp(runtime, argument ?? UdpRemoteEndpoint),
            "serial" => runtime => RunSerial(runtime, argument ?? SerialPort),
            "tls-ca" => runtime => RunTlsServer(runtime, argument ?? TlsEndpoint, GetCaTlsConfig()),
            "tls-self-signed" => runtime => RunTlsServer(runtime, argument ?? TlsEndpoint, GetSelfSignedTlsConfig()),
            _ => null,
        };
        if (run == null)
        {
            Console.WriteLine("Usage: outstation <tcp-server|tcp-client|udp|serial|tls-ca|tls-self-signed> [endpoint|serial-port]");
            return 1;
        }

        // Initialize logging with the default configuration
        Logging.Configure(new LoggingConfig(), new ConsoleLogger());

        var runtime = new Runtime(new RuntimeConfig { NumCoreThreads = 4 });
        try
        {
            run(runtime);
            return 0;
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
            return 1;
        }
        finally
        {
            // shuts down the server and outstation
            runtime.Shutdown();
        }
    }

    private static void RunTcpServer(Runtime runtime, string endpoint)
    {
        // ANCHOR: create_tcp_server
        var server = OutstationServer.CreateTcpServer(runtime, LinkErrorMode.Close, endpoint);
        // ANCHOR_END: create_tcp_server

        try
        {
            RunServer(server);
        }
        finally
        {
            // shutting the server down here also keeps it referenced (not finalized) while the console loop runs
            server.Shutdown();
        }
    }

    private static void RunTcpClient(Runtime runtime, string endpoint)
    {
        var outstation = Outstation.CreateTcpClient(
            runtime,
            LinkErrorMode.Close,
            new EndpointList(endpoint),
            new ConnectStrategy(),
            new ConnectOptions(),
            GetOutstationConfig(),
            new ExampleOutstationApplication(),
            new ExampleOutstationInformation(),
            new ExampleControlHandler(),
            new LogListener("client")
        );

        InitializeDatabase(outstation);
        // a TCP client outstation starts disabled: enable it to start connecting
        outstation.Enable();
        RunOutstation(outstation);
    }

    private static void RunUdp(Runtime runtime, string remoteEndpoint)
    {
        // ANCHOR: create_udp
        var config = new OutstationUdpConfig(UdpLocalEndpoint, remoteEndpoint);

        var outstation = Outstation.CreateUdp(
            runtime,
            config,
            GetOutstationConfig(),
            new ExampleOutstationApplication(),
            new ExampleOutstationInformation(),
            new ExampleControlHandler()
        );
        // ANCHOR_END: create_udp

        InitializeDatabase(outstation);
        RunOutstation(outstation);
    }

    private static void RunSerial(Runtime runtime, string port)
    {
        // ANCHOR: create_serial_server
        var outstation = Outstation.CreateSerialSession2(
            runtime,
            port,
            new SerialSettings(), // 9600 8N1, no flow control
            TimeSpan.FromSeconds(1), // try to open the port every second
            GetOutstationConfig(),
            // customizable interface that controls outstation behavior
            new ExampleOutstationApplication(),
            // customizable interface to receive events about what the outstation is doing
            new ExampleOutstationInformation(),
            // customizable interface to process control requests from the master
            new ExampleControlHandler(),
            new LogListener("port")
        );
        // ANCHOR_END: create_serial_server

        InitializeDatabase(outstation);
        RunOutstation(outstation);
    }

    private static void RunTlsServer(Runtime runtime, string endpoint, TlsServerConfig config)
    {
        // ANCHOR: create_tls_server
        var server = OutstationServer.CreateTlsServer(runtime, LinkErrorMode.Close, endpoint, config);
        // ANCHOR_END: create_tls_server

        try
        {
            RunServer(server);
        }
        finally
        {
            // shutting the server down here also keeps it referenced (not finalized) while the console loop runs
            server.Shutdown();
        }
    }

    private static void RunServer(OutstationServer server)
    {
        // ANCHOR: tcp_server_add_outstation
        var outstation = server.AddOutstation(
            GetOutstationConfig(),
            new ExampleOutstationApplication(),
            new ExampleOutstationInformation(),
            new ExampleControlHandler(),
            new LogListener("connection"),
            AddressFilter.Any()
        );
        // ANCHOR_END: tcp_server_add_outstation

        // set up the outstation's database before the server starts accepting connections
        InitializeDatabase(outstation);

        // ANCHOR: tcp_server_bind
        server.Bind();
        // ANCHOR_END: tcp_server_bind

        RunOutstation(outstation);
    }

    private static OutstationConfig GetOutstationConfig()
    {
        // ANCHOR: outstation_config
        // create an outstation configuration with default values
        var config = new OutstationConfig(
            // outstation address
            OutstationAddress,
            // master address
            MasterAddress,
            // event buffer sizes
            GetEventBufferConfig()
        ).WithDecodeLevel(DecodeLevel.Nothing().WithApplication(AppDecodeLevel.ObjectValues));
        // ANCHOR_END: outstation_config
        return config;
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

    private static TlsServerConfig GetCaTlsConfig()
    {
        // ANCHOR: tls_ca_chain_config
        var config = new TlsServerConfig(
            TlsDomain,
            CaCert,
            CaEntityCert,
            CaEntityKey,
            "" // no password
        );
        // ANCHOR_END: tls_ca_chain_config
        return config;
    }

    private static TlsServerConfig GetSelfSignedTlsConfig()
    {
        // ANCHOR: tls_self_signed_config
        var config = new TlsServerConfig(
            TlsDomain,
            SelfSignedPeerCert,
            SelfSignedEntityCert,
            SelfSignedEntityKey,
            "" // no password
        ).WithCertificateMode(CertificateMode.SelfSigned);
        // ANCHOR_END: tls_self_signed_config
        return config;
    }

    // Add the example's points and attributes to the database
    private static void InitializeDatabase(Outstation outstation)
    {
        // ANCHOR: database_init
        outstation.Transaction(db =>
        {
            // initialize PointCount points of each type
            for (ushort i = 0; i < PointCount; i++)
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
                db.AddAnalogInput(i, EventClass.Class1,
                    new AnalogInputConfig(StaticAnalogInputVariation.Group30Var1, EventAnalogInputVariation.Group32Var1, 0.0)
                );
                db.AddAnalogOutputStatus(i, EventClass.Class1, new AnalogOutputStatusConfig());
                db.AddOctetString(i, EventClass.Class1);
            }

            // define device attributes made available to the master
            db.DefineStringAttr(0, false, AttributeVariations.DeviceManufacturersName, "Step Function I/O");
            db.DefineStringAttr(0, true, AttributeVariations.UserAssignedLocation, "Bend, OR");
        });
        // ANCHOR_END: database_init
    }

    // run the same logic regardless of the transport type
    private static void RunOutstation(Outstation outstation)
    {
        var binaryInputValue = false;
        var doubleBitBinaryInputValue = DoubleBit.DeterminedOff;
        var binaryOutputStatusValue = false;
        var analogInputValue = 0.0;
        var analogOutputStatusValue = 0.0;
        var online = new Flags(Flag.Online);
        var detectEvent = UpdateOptions.DetectEvent();

        PrintHelp();

        // exit on "x" or at the end of input
        while (true)
        {
            var line = Console.ReadLine();
            if (line == null || line == "x")
            {
                return;
            }

            if (line == "help")
            {
                PrintHelp();
            }
            else if (line == "enable")
            {
                outstation.Enable();
            }
            else if (line == "disable")
            {
                outstation.Disable();
            }
            else if (line == "bi")
            {
                binaryInputValue = !binaryInputValue;
                outstation.Transaction(db =>
                    db.UpdateBinaryInput(new BinaryInput(7, binaryInputValue, online, Now()), detectEvent)
                );
            }
            else if (line == "dbbi")
            {
                doubleBitBinaryInputValue = doubleBitBinaryInputValue == DoubleBit.DeterminedOff ? DoubleBit.DeterminedOn : DoubleBit.DeterminedOff;
                outstation.Transaction(db =>
                    db.UpdateDoubleBitBinaryInput(new DoubleBitBinaryInput(7, doubleBitBinaryInputValue, online, Now()), detectEvent)
                );
            }
            else if (line == "bos")
            {
                binaryOutputStatusValue = !binaryOutputStatusValue;
                outstation.Transaction(db =>
                    db.UpdateBinaryOutputStatus(new BinaryOutputStatus(7, binaryOutputStatusValue, online, Now()), detectEvent)
                );
            }
            else if (line == "co")
            {
                outstation.Transaction(db =>
                {
                    // read the counter, which a freeze may have cleared
                    var value = db.GetCounter(7).Value;
                    db.UpdateCounter(new Counter(7, unchecked(value + 1), online, Now()), detectEvent);
                });
            }
            else if (line == "fco")
            {
                outstation.Transaction(db =>
                {
                    // read the frozen counter, which a freeze may have changed
                    var value = db.GetFrozenCounter(7).Value;
                    db.UpdateFrozenCounter(new FrozenCounter(7, unchecked(value + 1), online, Now()), detectEvent);
                });
            }
            else if (line == "ai")
            {
                analogInputValue += 1.0;
                outstation.Transaction(db =>
                    db.UpdateAnalogInput(new AnalogInput(7, analogInputValue, online, Now()), detectEvent)
                );
            }
            else if (line == "aif")
            {
                outstation.Transaction(db =>
                    db.UpdateFlags(7, UpdateFlagsType.AnalogInput, new Flags(Flag.CommLost), Now(), detectEvent)
                );
            }
            else if (line == "aos")
            {
                analogOutputStatusValue += 1.0;
                outstation.Transaction(db =>
                    db.UpdateAnalogOutputStatus(new AnalogOutputStatus(7, analogOutputStatusValue, online, Now()), detectEvent)
                );
            }
            else if (line == "os")
            {
                outstation.Transaction(db =>
                    db.UpdateOctetString(7, System.Text.Encoding.ASCII.GetBytes("Hello"), detectEvent)
                );
            }
            else
            {
                Console.WriteLine($"unknown command: {line}");
            }
        }
    }

    private static void PrintHelp()
    {
        Console.WriteLine("Commands (each updates the point at index 7):");
        Console.WriteLine("  help - print this list");
        Console.WriteLine("  x - exit");
        Console.WriteLine("  enable - enable the outstation");
        Console.WriteLine("  disable - disable the outstation");
        Console.WriteLine("  bi - toggle binary input");
        Console.WriteLine("  dbbi - toggle double bit binary input");
        Console.WriteLine("  bos - toggle binary output status");
        Console.WriteLine("  co - increment counter");
        Console.WriteLine("  fco - increment frozen counter");
        Console.WriteLine("  ai - increment analog input");
        Console.WriteLine("  aif - set analog input flag to COMM_LOST");
        Console.WriteLine("  aos - increment analog output status");
        Console.WriteLine("  os - set octet string to 'Hello'");
    }

    class ConsoleLogger : ILogger
    {
        public void OnMessage(LogLevel level, string message)
        {
            Console.Write(message);
        }
    }

    // C# interfaces have no default methods, so this class implements every callback;
    // the ones after the attribute writes keep the default behavior
    class ExampleOutstationApplication : IOutstationApplication
    {
        // ANCHOR: time_sync
        // ask the master for the time as soon as it connects
        private bool needTime = true;

        public ApplicationIin GetApplicationIin()
        {
            return new ApplicationIin { NeedTime = needTime };
        }

        public WriteTimeResult WriteAbsoluteTime(ulong time)
        {
            // a real device sets its clock here, and sets NeedTime again when its clock drifts
            Console.WriteLine($"write absolute time: {time}");
            needTime = false;
            return WriteTimeResult.Ok;
        }
        // ANCHOR_END: time_sync

        // ANCHOR: freeze
        public FreezeResult FreezeCountersAll(FreezeType freezeType, DatabaseHandle database)
        {
            return FreezeCounters(0, PointCount - 1, freezeType, database);
        }

        public FreezeResult FreezeCountersRange(ushort start, ushort stop, FreezeType freezeType, DatabaseHandle database)
        {
            return FreezeCounters(start, Math.Min((int)stop, PointCount - 1), freezeType, database);
        }

        // freezing at a time and interval isn't supported
        public FreezeResult FreezeCountersAllAtTime(DatabaseHandle database, ulong time, uint interval)
        {
            return FreezeResult.NotSupported;
        }

        public FreezeResult FreezeCountersRangeAtTime(ushort start, ushort stop, DatabaseHandle database, ulong time, uint interval)
        {
            return FreezeResult.NotSupported;
        }

        private static FreezeResult FreezeCounters(int start, int stop, FreezeType freezeType, DatabaseHandle database)
        {
            var clear = freezeType == FreezeType.FreezeAndClear;
            Console.WriteLine($"freeze counters {start} to {stop} (clear: {(clear ? "true" : "false")})");
            database.Transaction(db =>
            {
                for (var i = start; i <= stop; i++)
                {
                    var index = (ushort)i;
                    try
                    {
                        var counter = db.GetCounter(index);
                        var time = Now();
                        db.UpdateFrozenCounter(new FrozenCounter(index, counter.Value, counter.Flags, time), UpdateOptions.DetectEvent());
                        if (clear)
                        {
                            db.UpdateCounter(new Counter(index, 0, counter.Flags, time), UpdateOptions.DetectEvent());
                        }
                    }
                    catch (ParamException)
                    {
                        // skip a point the database doesn't have (yet): an exception must not escape the callback
                    }
                }
            });
            return FreezeResult.Ok;
        }
        // ANCHOR_END: freeze

        // ANCHOR: save_changes
        public bool SupportWriteAnalogDeadBands()
        {
            return true;
        }

        public void BeginWriteAnalogDeadBands()
        {
            Console.WriteLine("begin writing analog dead-bands");
        }

        public void WriteAnalogDeadBand(ushort index, double deadBand)
        {
            Console.WriteLine($"change analog dead-band {index} to {deadBand}");
        }

        public void EndWriteAnalogDeadBands()
        {
            Console.WriteLine("end writing analog dead-bands: save them here");
        }

        public bool SupportAssignClass()
        {
            return true;
        }

        public void BeginAssignClass()
        {
            Console.WriteLine("begin assigning classes");
        }

        public void AssignClassAll(PointType pointType, EventClass pointClass)
        {
            Console.WriteLine($"assign class {pointClass} to all {pointType} points");
        }

        public void AssignClassRange(PointType pointType, ushort start, ushort stop, EventClass pointClass)
        {
            Console.WriteLine($"assign class {pointClass} to {pointType} points {start} to {stop}");
        }

        public void EndAssignClass()
        {
            Console.WriteLine("end assigning classes: save them here");
        }
        // ANCHOR_END: save_changes

        public bool WriteStringAttr(byte set, byte variation, StringAttr attrType, string value)
        {
            Console.WriteLine($"write device attribute: {attrType} set={set} variation={variation} value={value}");
            // allow writing any string attribute that has been defined as writable
            return true;
        }

        public ushort GetProcessingDelayMs() { return 0; }
        public RestartDelay ColdRestart() { return RestartDelay.NotSupported(); }
        public RestartDelay WarmRestart() { return RestartDelay.NotSupported(); }
        public bool WriteFloatAttr(byte set, byte variation, FloatAttr attrType, float value) { return false; }
        public bool WriteDoubleAttr(byte set, byte variation, FloatAttr attrType, double value) { return false; }
        public bool WriteUintAttr(byte set, byte variation, UintAttr attrType, uint value) { return false; }
        public bool WriteIntAttr(byte set, byte variation, IntAttr attrType, int value) { return false; }
        public bool WriteOctetStringAttr(byte set, byte variation, OctetStringAttr attrType, ICollection<byte> value) { return false; }
        public bool WriteBitStringAttr(byte set, byte variation, BitStringAttr attrType, ICollection<byte> value) { return false; }
        public bool WriteTimeAttr(byte set, byte variation, TimeAttr attrType, ulong value) { return false; }
        public void BeginConfirm() { }
        public void EventCleared(ulong id) { }
        public void EndConfirm(BufferState state) { }
    }

    // ANCHOR: control_handler
    class ExampleControlHandler : IControlHandler
    {
        public void BeginFragment() { }

        public void EndFragment(DatabaseHandle database) { }

        public CommandStatus SelectG12v1(Group12Var1 control, ushort index, DatabaseHandle database)
        {
            if (index < PointCount && (control.Code.OpType == OpType.LatchOn || control.Code.OpType == OpType.LatchOff))
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
            Console.WriteLine($"operate {control.Code.OpType} on binary output {index} via {opType}");
            if (index < PointCount && (control.Code.OpType == OpType.LatchOn || control.Code.OpType == OpType.LatchOff))
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
            return index < PointCount ? CommandStatus.Success : CommandStatus.NotSupported;
        }

        private CommandStatus OperateAnalogOutput(double value, ushort index, DatabaseHandle database)
        {
            if (index < PointCount)
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

    // C# interfaces have no default methods: these keep the default behavior (do nothing)
    class ExampleOutstationInformation : IOutstationInformation
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

    // Logs each state change of a connection, client or serial port
    class LogListener : IConnectionStateListener, IClientStateListener, IPortStateListener
    {
        private readonly string name;

        public LogListener(string name)
        {
            this.name = name;
        }

        public void OnChange(ConnectionState state)
        {
            Console.WriteLine($"{name} state: {state}");
        }

        public void OnChange(ClientState state)
        {
            Console.WriteLine($"{name} state: {state}");
        }

        public void OnChange(PortState state)
        {
            Console.WriteLine($"{name} state: {state}");
        }
    }

    private static Timestamp Now()
    {
        return Timestamp.SynchronizedTimestamp((ulong)DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
    }
}
