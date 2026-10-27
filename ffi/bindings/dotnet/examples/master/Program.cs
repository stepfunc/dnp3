// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using dnp3;

/*
  Example program using the master API.

  The program initializes a master channel based on the command line argument and then enters a loop
  reading console input allowing the user to perform common tasks interactively.
*/
class ExampleMaster
{
    const ushort MasterAddress = 1;
    const ushort OutstationAddress = 1024;
    const string TcpEndpoint = "127.0.0.1:20000";
    const string TlsEndpoint = "127.0.0.1:20001";
    const string UdpLocalEndpoint = "127.0.0.1:20001";
    const string UdpRemoteEndpoint = "127.0.0.1:20000";
    const string SerialPort = "/dev/ttyS0";
    const string TlsDomain = "test.com";
    const string CaCert = "./certs/ca_chain/ca_cert.pem";
    const string CaEntityCert = "./certs/ca_chain/entity1_cert.pem";
    const string CaEntityKey = "./certs/ca_chain/entity1_key.pem";
    const string SelfSignedPeerCert = "./certs/self_signed/entity2_cert.pem";
    const string SelfSignedEntityCert = "./certs/self_signed/entity1_cert.pem";
    const string SelfSignedEntityKey = "./certs/self_signed/entity1_key.pem";

    const string Success = "success";

    public static async Task<int> Main(string[] args)
    {
        // the transport plus an optional endpoint (TCP/TLS), remote endpoint (UDP) or serial port
        var argument = args.Length == 2 ? args[1] : null;
        Func<Runtime, (MasterChannel, AssociationId)> createChannelAndAssociation = args.Length < 1 || args.Length > 2 ? null : args[0] switch
        {
            "tcp-client" => runtime => AddAssociation(CreateTcpChannel(runtime, argument ?? TcpEndpoint)),
            "udp" => runtime => AddUdpAssociation(CreateUdpChannel(runtime), argument ?? UdpRemoteEndpoint),
            "serial" => runtime => AddAssociation(CreateSerialChannel(runtime, argument ?? SerialPort)),
            "tls-ca" => runtime => AddAssociation(CreateTlsChannel(runtime, argument ?? TlsEndpoint, GetCaTlsConfig())),
            "tls-self-signed" => runtime => AddAssociation(CreateTlsChannel(runtime, argument ?? TlsEndpoint, GetSelfSignedTlsConfig())),
            _ => null,
        };
        if (createChannelAndAssociation == null)
        {
            Console.WriteLine("Usage: master <tcp-client|udp|serial|tls-ca|tls-self-signed> [endpoint|serial-port]");
            return 1;
        }

        // ANCHOR: logging_init
        // Initialize logging with the default configuration
        // This may only be called once during program initialization
        Logging.Configure(
            new LoggingConfig(),
            new ConsoleLogger()
        );
        // ANCHOR_END: logging_init

        // ANCHOR: runtime_init
        var runtime = new Runtime(new RuntimeConfig { NumCoreThreads = 4 });
        // ANCHOR_END: runtime_init

        try
        {
            var (channel, association) = createChannelAndAssociation(runtime);

            // create an event poll
            // ANCHOR: add_poll
            var poll = channel.AddPoll(association, Request.ClassRequest(false, true, true, true), TimeSpan.FromSeconds(5));
            // ANCHOR_END: add_poll

            // enable communications
            channel.Enable();

            PrintHelp();

            // exit on "x" or at the end of input
            while (true)
            {
                var line = Console.ReadLine();
                if (line == null || line == "x")
                {
                    return 0;
                }

                if (line == "help")
                {
                    PrintHelp();
                }
                else
                {
                    try
                    {
                        var outcome = await RunOneCommand(channel, association, poll, line);
                        Console.WriteLine($"{line}: {outcome}");
                    }
                    catch (Exception ex)
                    {
                        Console.WriteLine($"{line}: {ex.Message}");
                    }
                }
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
            return 1;
        }
        finally
        {
            // ANCHOR: runtime_shutdown
            runtime.Shutdown();
            // ANCHOR_END: runtime_shutdown
        }
    }

    private static MasterChannel CreateTcpChannel(Runtime runtime, string endpoint)
    {
        // ANCHOR: create_tcp_channel
        var channel = MasterChannel.CreateTcpChannel(
            runtime,
            LinkErrorMode.Close,
            GetMasterChannelConfig(),
            new EndpointList(endpoint),
            new ConnectStrategy(),
            new LogListener("client")
        );
        // ANCHOR_END: create_tcp_channel
        return channel;
    }

    private static MasterChannel CreateUdpChannel(Runtime runtime)
    {
        // ANCHOR: create_udp_channel
        var channel = MasterChannel.CreateUdpChannel(
            runtime,
            GetMasterChannelConfig(),
            UdpLocalEndpoint,
            LinkReadMode.Datagram,
            TimeSpan.FromSeconds(5)
        );
        // ANCHOR_END: create_udp_channel
        return channel;
    }

    private static MasterChannel CreateSerialChannel(Runtime runtime, string port)
    {
        // ANCHOR: create_serial_channel
        var channel = MasterChannel.CreateSerialChannel(
            runtime,
            GetMasterChannelConfig(),
            port,
            new SerialSettings(), // 9600 8N1, no flow control
            TimeSpan.FromSeconds(1), // try to open the port every second
            new LogListener("port")
        );
        // ANCHOR_END: create_serial_channel
        return channel;
    }

    private static MasterChannel CreateTlsChannel(Runtime runtime, string endpoint, TlsClientConfig config)
    {
        // ANCHOR: create_tls_channel
        var channel = MasterChannel.CreateTlsChannel(
            runtime,
            LinkErrorMode.Close,
            GetMasterChannelConfig(),
            new EndpointList(endpoint),
            new ConnectStrategy(),
            new LogListener("client"),
            config
        );
        // ANCHOR_END: create_tls_channel
        return channel;
    }

    private static (MasterChannel, AssociationId) AddAssociation(MasterChannel channel)
    {
        // ANCHOR: association_create
        var association = channel.AddAssociation(
            OutstationAddress,
            GetAssociationConfig(),
            new ExampleReadHandler(),
            new ExampleAssociationHandler(),
            new ExampleAssociationInformation()
        );
        // ANCHOR_END: association_create
        return (channel, association);
    }

    private static (MasterChannel, AssociationId) AddUdpAssociation(MasterChannel channel, string remoteEndpoint)
    {
        // ANCHOR: association_create_udp
        var association = channel.AddUdpAssociation(
            OutstationAddress,
            remoteEndpoint,
            GetAssociationConfig(),
            new ExampleReadHandler(),
            new ExampleAssociationHandler(),
            new ExampleAssociationInformation()
        );
        // ANCHOR_END: association_create_udp
        return (channel, association);
    }

    // Run a command and describe its outcome
    private static async Task<string> RunOneCommand(MasterChannel channel, AssociationId association, PollId poll, string command)
    {
        switch (command)
        {
            case "enable":
                {
                    channel.Enable();
                    return Success;
                }
            case "disable":
                {
                    channel.Disable();
                    return Success;
                }
            case "dln":
                {
                    channel.SetDecodeLevel(DecodeLevel.Nothing());
                    return Success;
                }
            case "dlv":
                {
                    channel.SetDecodeLevel(DecodeLevel.Nothing().WithApplication(AppDecodeLevel.ObjectValues));
                    return Success;
                }
            case "rao":
                {
                    var request = new Request();
                    request.AddAllObjectsHeader(Variation.Group40Var0);
                    await channel.Read(association, request);
                    return Success;
                }
            case "rmo":
                {
                    var request = new Request();
                    request.AddAllObjectsHeader(Variation.Group10Var0);
                    request.AddAllObjectsHeader(Variation.Group40Var0);
                    await channel.Read(association, request);
                    return Success;
                }
            case "rco":
                {
                    var request = new Request();
                    request.AddAllObjectsHeader(Variation.Group20Var0);
                    request.AddAllObjectsHeader(Variation.Group21Var0);
                    await channel.Read(association, request);
                    return Success;
                }
            case "cmd":
                {
                    // ANCHOR: assoc_control
                    var commands = new CommandSet();
                    commands.AddG12V1U16(3, Group12Var1.FromCode(ControlCode.FromOpType(OpType.LatchOn)));
                    await channel.Operate(association, CommandMode.SelectBeforeOperate, commands);
                    // ANCHOR_END: assoc_control
                    return Success;
                }
            case "cna":
                {
                    // ANCHOR: assoc_control_no_ack
                    var commands = new CommandSet();
                    commands.AddG12V1U16(3, Group12Var1.FromCode(ControlCode.FromOpType(OpType.LatchOn)));
                    await channel.OperateNoAck(association, commands);
                    // ANCHOR_END: assoc_control_no_ack
                    // success only means the request was sent: the outstation doesn't answer
                    return "sent";
                }
            case "evt":
                {
                    // the poll runs later, and its data goes to the read handler
                    channel.DemandPoll(poll);
                    return "poll demanded";
                }
            case "lts":
                {
                    await channel.SynchronizeTime(association, TimeSyncMode.Lan);
                    return Success;
                }
            case "nts":
                {
                    await channel.SynchronizeTime(association, TimeSyncMode.NonLan);
                    return Success;
                }
            case "wad":
                {
                    // ANCHOR: write_dead_bands
                    var request = new WriteDeadBandRequest();
                    request.AddG34v1U8(3, 5);
                    request.AddG34v3U16(4, 2.5f);
                    await channel.WriteDeadBands(association, request);
                    // ANCHOR_END: write_dead_bands
                    return Success;
                }
            case "fat":
                {
                    // ANCHOR: freeze_at_time
                    var request = new Request();
                    // freeze all the counters once per day relative to the beginning of the current hour
                    request.AddTimeAndInterval(0, 86400000);
                    // apply this schedule to all counters
                    request.AddAllObjectsHeader(Variation.Group20Var0);
                    await channel.SendAndExpectEmptyResponse(association, FunctionCode.FreezeAtTime, request);
                    // ANCHOR_END: freeze_at_time
                    return Success;
                }
            case "ifz":
                {
                    // copy every counter to its frozen counter
                    var request = new Request();
                    request.AddAllObjectsHeader(Variation.Group20Var0);
                    await channel.SendAndExpectEmptyResponse(association, FunctionCode.ImmediateFreeze, request);
                    return Success;
                }
            case "fcl":
                {
                    // copy every counter to its frozen counter, then clear the counter
                    var request = new Request();
                    request.AddAllObjectsHeader(Variation.Group20Var0);
                    await channel.SendAndExpectEmptyResponse(association, FunctionCode.FreezeClear, request);
                    return Success;
                }
            case "rda":
                {
                    // ANCHOR: read_attributes
                    var request = new Request();
                    request.AddSpecificAttribute(AttributeVariations.AllAttributesRequest, 0);
                    await channel.Read(association, request);
                    // ANCHOR_END: read_attributes
                    return Success;
                }
            case "wda":
                {
                    // ANCHOR: write_attribute
                    var request = new Request();
                    request.AddStringAttribute(AttributeVariations.UserAssignedLocation, 0, "Mt. Olympus");
                    await channel.SendAndExpectEmptyResponse(association, FunctionCode.Write, request);
                    // ANCHOR_END: write_attribute
                    return Success;
                }
            case "ral":
                {
                    var request = new Request();
                    request.AddSpecificAttribute(AttributeVariations.ListOfVariations, 0);
                    await channel.Read(association, request);
                    return Success;
                }
            case "crt":
                {
                    var delay = await channel.ColdRestart(association);
                    Console.WriteLine($"restart delay: {delay}");
                    return Success;
                }
            case "wrt":
                {
                    var delay = await channel.WarmRestart(association);
                    Console.WriteLine($"restart delay: {delay}");
                    return Success;
                }
            case "rd":
                {
                    // ANCHOR: read_directory
                    var items = await channel.ReadDirectory(association, ".", DirReadConfig.Defaults());
                    foreach (var info in items)
                    {
                        PrintFileInfo(info);
                    }
                    // ANCHOR_END: read_directory
                    return Success;
                }
            case "gfi":
                {
                    // ANCHOR: get_file_info
                    var info = await channel.GetFileInfo(association, ".");
                    PrintFileInfo(info);
                    // ANCHOR_END: get_file_info
                    return Success;
                }
            case "rf":
                {
                    // ANCHOR: read_file
                    // this reads the root "directory" file but doesn't parse it
                    channel.ReadFile(association, ".", FileReadConfig.Defaults(), new FileLogger());
                    // ANCHOR_END: read_file
                    // the read continues in the background: the file reader's Completed and Aborted callbacks report its outcome
                    return "started (the file reader reports the outcome)";
                }
            case "wf":
                {
                    // ANCHOR: write_file
                    var line = System.Text.Encoding.UTF8.GetBytes("hello world\n");
                    var file = await channel.OpenFile(
                        association,
                        "hello_world.txt",
                        0, // no authentication key
                        Permissions.None(),
                        0xFFFFFFFF, // indicate that we don't know the size
                        FileMode.Write,
                        512
                    );

                    // write the 'hello world' line to the file twice
                    await channel.WriteFileBlock(association, file.FileHandle, 0, false, line);
                    await channel.WriteFileBlock(association, file.FileHandle, 1, true, line);
                    await channel.CloseFile(association, file.FileHandle);
                    // ANCHOR_END: write_file
                    return Success;
                }
            case "lsr":
                {
                    await channel.CheckLinkStatus(association);
                    return Success;
                }
            default:
                throw new ArgumentException("unknown command, type help for a list");
        }
    }

    private static void PrintHelp()
    {
        Console.WriteLine("Commands:");
        Console.WriteLine("  help - print this list");
        Console.WriteLine("  x - exit");
        Console.WriteLine("  enable / disable - enable or disable the channel");
        Console.WriteLine("  dln / dlv - decode nothing / decode object values");
        Console.WriteLine("  rao - read analog output statuses");
        Console.WriteLine("  rmo - read binary and analog output statuses");
        Console.WriteLine("  rco - read counters and frozen counters");
        Console.WriteLine("  cmd - select and operate binary output 3 (latch on)");
        Console.WriteLine("  cna - operate binary output 3 without an answer");
        Console.WriteLine("  evt - demand the event poll");
        Console.WriteLine("  lts / nts - synchronize the time (LAN / non-LAN)");
        Console.WriteLine("  wad - write analog input deadbands");
        Console.WriteLine("  ifz - freeze all counters");
        Console.WriteLine("  fcl - freeze and clear all counters");
        Console.WriteLine("  fat - freeze all counters once a day");
        Console.WriteLine("  rda - read all device attributes");
        Console.WriteLine("  wda - write the user-assigned location attribute");
        Console.WriteLine("  ral - read the list of attribute variations");
        Console.WriteLine("  crt / wrt - cold / warm restart");
        Console.WriteLine("  rd - read the root directory");
        Console.WriteLine("  gfi - get information about the root directory");
        Console.WriteLine("  rf - read the root directory as a file");
        Console.WriteLine("  wf - write hello_world.txt");
        Console.WriteLine("  lsr - check the link status");
    }

    // ANCHOR: master_channel_config
    private static MasterChannelConfig GetMasterChannelConfig()
    {
        return new MasterChannelConfig(MasterAddress)
            .WithDecodeLevel(DecodeLevel.Nothing().WithApplication(AppDecodeLevel.ObjectValues));
    }
    // ANCHOR_END: master_channel_config

    // ANCHOR: association_config
    private static AssociationConfig GetAssociationConfig()
    {
        return new AssociationConfig(
            // disable unsolicited first (Class 1/2/3)
            EventClasses.All(),
            // after the integrity poll, enable unsolicited (Class 1/2/3)
            EventClasses.All(),
            // perform startup integrity poll with Class 1/2/3/0
            Classes.All(),
            // don't automatically scan Class 1/2/3 when the corresponding IIN bit is asserted
            EventClasses.None()
        )
        .WithAutoTimeSync(AutoTimeSync.Lan)
        .WithKeepAliveTimeout(TimeSpan.FromSeconds(60));
    }
    // ANCHOR_END: association_config

    private static TlsClientConfig GetCaTlsConfig()
    {
        // ANCHOR: tls_ca_chain_config
        // defaults to CA mode
        var config = new TlsClientConfig(
            TlsDomain,
            CaCert,
            CaEntityCert,
            CaEntityKey,
            "" // no password
        );
        // ANCHOR_END: tls_ca_chain_config
        return config;
    }

    private static TlsClientConfig GetSelfSignedTlsConfig()
    {
        // ANCHOR: tls_self_signed_config
        var config = new TlsClientConfig(
            TlsDomain,
            SelfSignedPeerCert,
            SelfSignedEntityCert,
            SelfSignedEntityKey,
            "" // no password
        ).WithCertificateMode(CertificateMode.SelfSigned);
        // ANCHOR_END: tls_self_signed_config
        return config;
    }

    // ANCHOR: logging_interface
    // callback interface used to receive log messages
    class ConsoleLogger : ILogger
    {
        public void OnMessage(LogLevel level, string message)
        {
            Console.Write(message);
        }
    }
    // ANCHOR_END: logging_interface

    // ANCHOR: read_handler
    class ExampleReadHandler : IReadHandler
    {
        public void BeginFragment(ReadType readType, ResponseHeader header)
        {
            Console.WriteLine($"begin fragment: {readType} (broadcast: {FormatBool(header.Iin.Iin1.Broadcast)})");
        }

        public void EndFragment(ReadType readType, ResponseHeader header)
        {
            Console.WriteLine("end fragment");
        }

        public void HandleBinaryInput(HeaderInfo info, ICollection<BinaryInput> values)
        {
            PrintHeader("Binary inputs", info);
            foreach (var x in values)
            {
                PrintPoint("BI", x.Index, FormatBool(x.Value), x.Flags, x.Time);
            }
        }

        public void HandleDoubleBitBinaryInput(HeaderInfo info, ICollection<DoubleBitBinaryInput> values)
        {
            PrintHeader("Double-bit binary inputs", info);
            foreach (var x in values)
            {
                PrintPoint("DBBI", x.Index, x.Value, x.Flags, x.Time);
            }
        }

        public void HandleBinaryOutputStatus(HeaderInfo info, ICollection<BinaryOutputStatus> values)
        {
            PrintHeader("Binary output statuses", info);
            foreach (var x in values)
            {
                PrintPoint("BOS", x.Index, FormatBool(x.Value), x.Flags, x.Time);
            }
        }

        public void HandleCounter(HeaderInfo info, ICollection<Counter> values)
        {
            PrintHeader("Counters", info);
            foreach (var x in values)
            {
                PrintPoint("C", x.Index, x.Value, x.Flags, x.Time);
            }
        }

        public void HandleFrozenCounter(HeaderInfo info, ICollection<FrozenCounter> values)
        {
            PrintHeader("Frozen counters", info);
            foreach (var x in values)
            {
                PrintPoint("FC", x.Index, x.Value, x.Flags, x.Time);
            }
        }

        public void HandleAnalogInput(HeaderInfo info, ICollection<AnalogInput> values)
        {
            PrintHeader("Analog inputs", info);
            foreach (var x in values)
            {
                PrintPoint("AI", x.Index, x.Value, x.Flags, x.Time);
            }
        }

        public void HandleFrozenAnalogInput(HeaderInfo info, ICollection<FrozenAnalogInput> values)
        {
            PrintHeader("Frozen analog inputs", info);
            foreach (var x in values)
            {
                PrintPoint("FAI", x.Index, x.Value, x.Flags, x.Time);
            }
        }

        public void HandleAnalogInputDeadBand(HeaderInfo info, ICollection<AnalogInputDeadBand> values)
        {
            PrintHeader("Analog input deadbands", info);
            foreach (var x in values)
            {
                Console.WriteLine($"AIDB {x.Index}: value={x.Value}");
            }
        }

        public void HandleAnalogOutputStatus(HeaderInfo info, ICollection<AnalogOutputStatus> values)
        {
            PrintHeader("Analog output statuses", info);
            foreach (var x in values)
            {
                PrintPoint("AOS", x.Index, x.Value, x.Flags, x.Time);
            }
        }

        public void HandleBinaryOutputCommandEvent(HeaderInfo info, ICollection<BinaryOutputCommandEvent> values)
        {
            PrintHeader("Binary output command events", info);
            foreach (var x in values)
            {
                Console.WriteLine($"BOCE {x.Index}: commanded state={FormatBool(x.CommandedState)} status={x.Status} time={FormatTime(x.Time)}");
            }
        }

        public void HandleAnalogOutputCommandEvent(HeaderInfo info, ICollection<AnalogOutputCommandEvent> values)
        {
            PrintHeader("Analog output command events", info);
            foreach (var x in values)
            {
                Console.WriteLine($"AOCE {x.Index}: commanded value={x.CommandedValue} status={x.Status} time={FormatTime(x.Time)}");
            }
        }

        public void HandleUnsignedInteger(HeaderInfo info, ICollection<UnsignedInteger> values)
        {
            PrintHeader("Unsigned integers", info);
            foreach (var x in values)
            {
                Console.WriteLine($"UI {x.Index}: value={x.Value}");
            }
        }

        public void HandleOctetString(HeaderInfo info, ICollection<OctetString> values)
        {
            PrintHeader("Octet strings", info);
            foreach (var x in values)
            {
                Console.WriteLine($"OS {x.Index}: value={FormatBytes(x.Value)}");
            }
        }

        public void HandleStringAttr(HeaderInfo info, StringAttr attr, byte set, byte variation, string value)
        {
            PrintHeader("Device attributes", info);
            PrintAttribute(set, variation, value);
        }

        public void HandleVariationListAttr(HeaderInfo info, VariationListAttr attr, byte set, byte variation, ICollection<AttrItem> value)
        {
            PrintHeader("Device attributes", info);
            PrintAttribute(set, variation, $"[{string.Join(", ", value.Select(item => item.Variation + (item.Properties.IsWritable ? " (writable)" : "")))}]");
        }

        public void HandleUintAttr(HeaderInfo info, UintAttr attr, byte set, byte variation, uint value)
        {
            PrintHeader("Device attributes", info);
            PrintAttribute(set, variation, value);
        }

        public void HandleBoolAttr(HeaderInfo info, BoolAttr attr, byte set, byte variation, bool value)
        {
            PrintHeader("Device attributes", info);
            PrintAttribute(set, variation, FormatBool(value));
        }

        public void HandleIntAttr(HeaderInfo info, IntAttr attr, byte set, byte variation, int value)
        {
            PrintHeader("Device attributes", info);
            PrintAttribute(set, variation, value);
        }

        public void HandleTimeAttr(HeaderInfo info, TimeAttr attr, byte set, byte variation, ulong value)
        {
            PrintHeader("Device attributes", info);
            PrintAttribute(set, variation, value);
        }

        public void HandleFloatAttr(HeaderInfo info, FloatAttr attr, byte set, byte variation, double value)
        {
            PrintHeader("Device attributes", info);
            PrintAttribute(set, variation, value);
        }

        public void HandleOctetStringAttr(HeaderInfo info, OctetStringAttr attr, byte set, byte variation, ICollection<byte> value)
        {
            PrintHeader("Device attributes", info);
            PrintAttribute(set, variation, FormatBytes(value));
        }

        public void HandleBitStringAttr(HeaderInfo info, BitStringAttr attr, byte set, byte variation, ICollection<byte> value)
        {
            PrintHeader("Device attributes", info);
            PrintAttribute(set, variation, FormatBytes(value));
        }

        public void HandleAbsTime(HeaderInfo info, Timestamp time)
        {
            PrintHeader("Absolute time", info);
            Console.WriteLine($"time={time.Value}");
        }

        private static void PrintHeader(string name, HeaderInfo info)
        {
            Console.WriteLine($"{name} ({info.Qualifier}, {info.Variation})");
        }

        private static void PrintPoint(string name, ushort index, object value, Flags flags, Timestamp time)
        {
            Console.WriteLine($"{name} {index}: value={value} flags=0x{flags.Value:X2} time={FormatTime(time)}");
        }

        private static string FormatTime(Timestamp time)
        {
            switch (time.Quality)
            {
                case TimeQuality.SynchronizedTime:
                    return $"{time.Value} (synchronized)";
                case TimeQuality.UnsynchronizedTime:
                    return $"{time.Value} (unsynchronized)";
                default:
                    return "none";
            }
        }

        private static void PrintAttribute(byte set, byte variation, object value)
        {
            Console.WriteLine($"attribute set={set} variation={variation} value={value}");
        }

        private static string FormatBytes(ICollection<byte> bytes)
        {
            return $"[{string.Join(", ", bytes.Select(b => b.ToString("X2")))}]";
        }

        private static string FormatBool(bool value)
        {
            return value ? "true" : "false";
        }
    }
    // ANCHOR_END: read_handler

    // ANCHOR: association_handler
    class ExampleAssociationHandler : IAssociationHandler
    {
        // the system time in milliseconds since the epoch, used to synchronize the outstation's clock
        public UtcTimestamp GetCurrentTime()
        {
            return UtcTimestamp.Valid((ulong)DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
        }
    }
    // ANCHOR_END: association_handler

    // ANCHOR: association_information
    // C# interfaces have no default methods: these keep the default behavior (do nothing)
    class ExampleAssociationInformation : IAssociationInformation
    {
        public void TaskStart(TaskType taskType, FunctionCode functionCode, byte seq) { }
        public void TaskSuccess(TaskType taskType, FunctionCode functionCode, byte seq) { }
        public void TaskFail(TaskType taskType, TaskError error) { }
        public void UnsolicitedResponse(bool isDuplicate, byte seq) { }
    }
    // ANCHOR_END: association_information

    // ANCHOR: file_logger
    class FileLogger : IFileReader
    {
        public bool Opened(uint size)
        {
            Console.WriteLine($"File opened - size: {size}");
            return true;
        }

        public bool BlockReceived(uint blockNum, ICollection<byte> data)
        {
            Console.WriteLine($"Received block {blockNum} with size: {data.Count}");
            return true;
        }

        public void Aborted(FileError error)
        {
            Console.WriteLine($"rf: aborted: {error}");
        }

        public void Completed()
        {
            Console.WriteLine("rf: completed");
        }
    }
    // ANCHOR_END: file_logger

    // Logs each state change of a client connection or serial port
    class LogListener : IClientStateListener, IPortStateListener
    {
        private readonly string name;

        public LogListener(string name)
        {
            this.name = name;
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

    private static void PrintFileInfo(FileInfo info)
    {
        Console.WriteLine($"File name: {info.FileName}");
        Console.WriteLine($"     type: {info.FileType}");
        Console.WriteLine($"     size: {info.Size}");
        Console.WriteLine($"     created: {info.TimeCreated}");
        Console.WriteLine("     permissions:");
        Console.WriteLine($"         world: {FormatPermissions(info.Permissions.World)}");
        Console.WriteLine($"         group: {FormatPermissions(info.Permissions.Group)}");
        Console.WriteLine($"         owner: {FormatPermissions(info.Permissions.Owner)}");
    }

    private static string FormatPermissions(PermissionSet permissions)
    {
        return $"{(permissions.Read ? 'r' : '-')}{(permissions.Write ? 'w' : '-')}{(permissions.Execute ? 'x' : '-')}";
    }
}
