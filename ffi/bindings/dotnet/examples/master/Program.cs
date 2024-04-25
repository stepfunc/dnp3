using System;
using System.Threading.Tasks;
using System.Collections.Generic;
using dnp3;

class MainClass
{
    // ANCHOR: logging_interface
    // callback interface used to receive log messages
    class ConsoleLogger : ILogger
    {
        public void OnMessage(LogLevel level, string message)
        {
            Console.Write($"{message}");
        }
    }
    // ANCHOR_END: logging_interface

    class TestClientStateListener : IClientStateListener
    {
        public void OnChange(ClientState state)
        {
            Console.WriteLine(state);
        }
    }

    class TestPortStateListener : IPortStateListener
    {
        public void OnChange(PortState state)
        {
            Console.WriteLine(state);
        }
    }

    // ANCHOR: read_handler
    class TestReadHandler : IReadHandler
    {
        public void BeginFragment(ReadType readType, ResponseHeader header)
        {
            Console.WriteLine($"Beginning fragment (broadcast: {header.Iin.Iin1.Broadcast})");
        }

        public void EndFragment(ReadType readType, ResponseHeader header)
        {
            Console.WriteLine("End fragment");
        }

        public void HandleBinaryInput(HeaderInfo info, ICollection<BinaryInput> values)
        {
            Console.WriteLine("Binary Inputs:");
            Console.WriteLine("Qualifier: " + info.Qualifier);
            Console.WriteLine("Variation: " + info.Variation);

            foreach (var val in values)
            {
                Console.WriteLine($"BI {val.Index}: Value={val.Value} Flags={val.Flags.Value} Time={val.Time.Value} ({val.Time.Quality})");
            }
        }

        public void HandleDoubleBitBinaryInput(HeaderInfo info, ICollection<DoubleBitBinaryInput> values)
        {
            Console.WriteLine("Double Bit Binary Inputs:");
            Console.WriteLine("Qualifier: " + info.Qualifier);
            Console.WriteLine("Variation: " + info.Variation);

            foreach (var val in values)
            {
                Console.WriteLine($"DBBI {val.Index}: Value={val.Value} Flags={val.Flags.Value} Time={val.Time.Value} ({val.Time.Quality})");
            }
        }

        public void HandleBinaryOutputStatus(HeaderInfo info, ICollection<BinaryOutputStatus> values)
        {
            Console.WriteLine("Binary Output Statuses:");
            Console.WriteLine("Qualifier: " + info.Qualifier);
            Console.WriteLine("Variation: " + info.Variation);

            foreach (var val in values)
            {
                Console.WriteLine($"BOS {val.Index}: Value={val.Value} Flags={val.Flags.Value} Time={val.Time.Value} ({val.Time.Quality})");
            }
        }

        public void HandleCounter(HeaderInfo info, ICollection<Counter> values)
        {
            Console.WriteLine("Counters:");
            Console.WriteLine("Qualifier: " + info.Qualifier);
            Console.WriteLine("Variation: " + info.Variation);

            foreach (var val in values)
            {
                Console.WriteLine($"Counter {val.Index}: Value={val.Value} Flags={val.Flags.Value} Time={val.Time.Value} ({val.Time.Quality})");
            }
        }

        public void HandleFrozenCounter(HeaderInfo info, ICollection<FrozenCounter> values)
        {
            Console.WriteLine("Frozen Counters:");
            Console.WriteLine("Qualifier: " + info.Qualifier);
            Console.WriteLine("Variation: " + info.Variation);

            foreach (var val in values)
            {
                Console.WriteLine($"Frozen Counter {val.Index}: Value={val.Value} Flags={val.Flags.Value} Time={val.Time.Value} ({val.Time.Quality})");
            }
        }

        public void HandleAnalogInput(HeaderInfo info, ICollection<AnalogInput> values)
        {
            Console.WriteLine("Analog Inputs:");
            Console.WriteLine("Qualifier: " + info.Qualifier);
            Console.WriteLine("Variation: " + info.Variation);

            foreach (var val in values)
            {
                Console.WriteLine($"AI {val.Index}: Value={val.Value} Flags={val.Flags.Value} Time={val.Time.Value} ({val.Time.Quality})");
            }
        }

        public void HandleFrozenAnalogInput(HeaderInfo info, ICollection<FrozenAnalogInput> values)
        {
            Console.WriteLine("Frozen Analog Inputs:");
            Console.WriteLine("Qualifier: " + info.Qualifier);
            Console.WriteLine("Variation: " + info.Variation);

            foreach (var val in values)
            {
                Console.WriteLine($"AI {val.Index}: Value={val.Value} Flags={val.Flags.Value} Time={val.Time.Value} ({val.Time.Quality})");
            }
        }

        public void HandleAnalogOutputStatus(HeaderInfo info, ICollection<AnalogOutputStatus> values)
        {
            Console.WriteLine("Analog Output Statuses:");
            Console.WriteLine("Qualifier: " + info.Qualifier);
            Console.WriteLine("Variation: " + info.Variation);

            foreach (var val in values)
            {
                Console.WriteLine($"AOS {val.Index}: Value={val.Value} Flags={val.Flags.Value} Time={val.Time.Value} ({val.Time.Quality})");
            }
        }

        void IReadHandler.HandleBinaryOutputCommandEvent(HeaderInfo info, ICollection<BinaryOutputCommandEvent> values)
        {
            Console.WriteLine("Binary Output Command Events:");
            Console.WriteLine("Qualifier: " + info.Qualifier);
            Console.WriteLine("Variation: " + info.Variation);

            foreach (var val in values)
            {
                Console.WriteLine($"BOCE {val.Index}: Value={val.CommandedState} Status={val.Status} Time={val.Time.Value} ({val.Time.Quality})");
            }
        }

        void IReadHandler.HandleAnalogOutputCommandEvent(HeaderInfo info, ICollection<AnalogOutputCommandEvent> values)
        {
            Console.WriteLine("Analog Output Command Events:");
            Console.WriteLine("Qualifier: " + info.Qualifier);
            Console.WriteLine("Variation: " + info.Variation);

            foreach (var val in values)
            {
                Console.WriteLine($"AOCE {val.Index}: Value={val.CommandedValue} Type={val.CommandType} Status={val.Status} Time={val.Time.Value} ({val.Time.Quality})");
            }
        }

        void IReadHandler.HandleUnsignedInteger(HeaderInfo info, ICollection<UnsignedInteger> values)
        {
            Console.WriteLine("Unsigned Integers:");
            Console.WriteLine("Qualifier: " + info.Qualifier);
            Console.WriteLine("Variation: " + info.Variation);

            foreach (var val in values)
            {
                Console.WriteLine($"{val.Index}: Value={val.Value}");
            }
        }

        public void HandleOctetString(HeaderInfo info, ICollection<OctetString> values)
        {
            Console.WriteLine("Octet Strings:");
            Console.WriteLine("Qualifier: " + info.Qualifier);
            Console.WriteLine("Variation: " + info.Variation);

            foreach (var val in values)
            {
                Console.Write($"Octet String {val.Index}: Value=");
                foreach (var b in val.Value)
                {
                    Console.Write($"{b:X2} ");
                }
                Console.WriteLine();
            }
        }
        
        void IReadHandler.HandleStringAttr(HeaderInfo info, StringAttr attr, byte set, byte var, string value)
        {
            Console.WriteLine($"Visible string attribute: {attr} set: {set} variation: {var} value: {value}");
        }

        void IReadHandler.HandleUintAttr(HeaderInfo info, UintAttr attr, byte set, byte var, uint value)
        {
            Console.WriteLine($"Unsigned integer attribute: {attr} set: {set} variation: {var} value: {value}");
        }

        void IReadHandler.HandleBoolAttr(HeaderInfo info, BoolAttr attr, byte set, byte var, bool value)
        {
            Console.WriteLine($"Boolean attribute: {attr} set: {set} variation: {var} value: {value}");
        }

        void IReadHandler.HandleIntAttr(HeaderInfo info, IntAttr attr, byte set, byte var, int value)
        {
            Console.WriteLine($"Int attribute: {attr} set: {set} variation: {var} value: {value}");
        }

        void IReadHandler.HandleTimeAttr(HeaderInfo info, TimeAttr attr, byte set, byte var, ulong value)
        {
            Console.WriteLine($"Time attribute: {attr} set: {set} variation: {var} value: {value}");
        }

        void IReadHandler.HandleFloatAttr(HeaderInfo info, FloatAttr attr, byte set, byte var, double value)
        {
            Console.WriteLine($"Float attribute: {attr} set: {set} variation: {var} value: {value}");
        }

        void IReadHandler.HandleVariationListAttr(HeaderInfo info, VariationListAttr attr, byte set, byte var, ICollection<AttrItem> value)
        {
            Console.WriteLine($"Attribute variation list: {attr} set: {set} variation: {var}");
            foreach(var item in value) {
                Console.WriteLine($"variation: {item.Variation} writable: {item.Properties.IsWritable}");
            }
        }

        void IReadHandler.HandleOctetStringAttr(HeaderInfo info, OctetStringAttr attr, byte set, byte var, ICollection<byte> value)
        {
            Console.WriteLine($"Octet-string attribute: {attr} set: {set} variation: {var} length: {value.Count}");            
        }

        void IReadHandler.HandleBitStringAttr(HeaderInfo info, BitStringAttr attr, byte set, byte var, ICollection<byte> value)
        {
            Console.WriteLine($"Bit-string attribute: {attr} set: {set} variation: {var} length: {value.Count}");
        }

        
    }
    // ANCHOR_END: read_handler

    // ANCHOR: association_handler
    class TestAssociationHandler : IAssociationHandler
    {
        public UtcTimestamp GetCurrentTime()
        {
            return UtcTimestamp.Valid((ulong)DateTime.UtcNow.Subtract(DateTime.UnixEpoch).TotalMilliseconds);
        }
    }
    // ANCHOR_END: association_handler

    // ANCHOR: association_information
    class TestAssociationInformation : IAssociationInformation
    {
        public void TaskStart(TaskType taskType, FunctionCode fc, byte seq) { }
        public void TaskSuccess(TaskType taskType, FunctionCode fc, byte seq) { }
        public void TaskFail(TaskType taskType, TaskError error) { }
        public void UnsolicitedResponse(bool isDuplicate, byte seq) { }
    }
    // ANCHOR_END: association_information

    // ANCHOR: file_logger
    class FileReader : IFileReader
    {
        void IFileReader.Aborted(FileError error)
        {
            Console.WriteLine($"File transfer aborted: {error}");
        }

        bool IFileReader.BlockReceived(uint blockNum, ICollection<byte> data)
        {
            Console.WriteLine($"Received file block {blockNum} with size {data.Count}");
            return true;
        }

        void IFileReader.Completed()
        {
            Console.WriteLine($"File transfer completed");
        }

        bool IFileReader.Opened(uint size)
        {
            Console.WriteLine($"Outstation open file with size: ${size}");
            return true;
        }
    }
    // ANCHOR_END: file_logger

    // ANCHOR: master_channel_config
    private static MasterChannelConfig GetMasterChannelConfig()
    {
        return new MasterChannelConfig(1)
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

    private static void RunTcp(Runtime runtime)
    {
        // ANCHOR: create_tcp_channel
        var channel = MasterChannel.CreateTcpChannel(
            runtime,
            LinkErrorMode.Close,
            GetMasterChannelConfig(),
            new EndpointList("127.0.0.1:20000"),
            new ConnectStrategy(),
            new TestClientStateListener()
        );
        // ANCHOR_END: create_tcp_channel

        try
        {
            RunChannel(channel);
        }
        finally
        {
            channel.Shutdown();
        }
    }

    private static void RunTls(Runtime runtime, TlsClientConfig tlsConfig)
    {
        // ANCHOR: create_tls_channel
        var channel = MasterChannel.CreateTlsChannel(
            runtime,
            LinkErrorMode.Close,
            GetMasterChannelConfig(),
            new EndpointList("127.0.0.1:20001"),
            new ConnectStrategy(),
            new TestClientStateListener(),
            tlsConfig
        );
        // ANCHOR_END: create_tls_channel

        try
        {
            RunChannel(channel);
        }
        finally
        {
            channel.Shutdown();
        }
    }

    private static void RunUdp(Runtime runtime)
    {
        // ANCHOR: create_udp_channel
        var channel = MasterChannel.CreateUdpChannel(
            runtime,
            GetMasterChannelConfig(),
            "127.0.0.1:20001",
            LinkReadMode.Datagram,
            TimeSpan.FromSeconds(5)
        );
        // ANCHOR_END: create_serial_channel

        try
        {
            // ANCHOR: association_create_udp
            var association = channel.AddUdpAssociation(
                1024,
                "127.0.0.1:20000",
                GetAssociationConfig(),
                new TestReadHandler(),
                new TestAssociationHandler(),
                new TestAssociationInformation()
            );
            // ANCHOR_END: association_create_udp

            RunAssociation(channel, association);
        }
        finally
        {
            channel.Shutdown();
        }
    }

    private static void RunSerial(Runtime runtime)
    {
        // ANCHOR: create_serial_channel
        var channel = MasterChannel.CreateSerialChannel(
            runtime,
            GetMasterChannelConfig(),
            "COM1",
            new SerialSettings(),
            TimeSpan.FromSeconds(5),
            new TestPortStateListener()
        );
        // ANCHOR_END: create_serial_channel

        try
        {
            RunChannel(channel);
        }
        finally
        {
            channel.Shutdown();
        }
    }
    private static TlsClientConfig GetCaTlsConfig()
    {
        // ANCHOR: tls_ca_chain_config
        // defaults to CA mode
        var config = new TlsClientConfig(
            "test.com",
            "./certs/ca_chain/ca_cert.pem",
            "./certs/ca_chain/entity1_cert.pem",
            "./certs/ca_chain/entity1_key.pem",
            "" // no password
        );
        // ANCHOR_END: tls_ca_chain_config
        return config;
    }

    private static TlsClientConfig GetSelfSignedTlsConfig()
    {
        // ANCHOR: tls_self_signed_config
        var config = new TlsClientConfig(
            "test.com",
            "./certs/self_signed/entity2_cert.pem",
            "./certs/self_signed/entity1_cert.pem",
            "./certs/self_signed/entity1_key.pem",
            "" // no password
        ).WithCertificateMode(CertificateMode.SelfSigned);
        // ANCHOR_END: tls_self_signed_config
        return config;
    }

    public static void Main(string[] args)
    {
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

        if (args.Length != 1)
        {
            System.Console.WriteLine("You must specify the transport type");
            System.Console.WriteLine("Usage: master-example <transport> (tcp, serial, tls-ca, tls-self-signed)");
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
            // ANCHOR: runtime_shutdown
            runtime.Shutdown();
            // ANCHOR_END: runtime_shutdown
        }
    }

    private static void RunAssociation(MasterChannel channel, AssociationId association)
    {
        // ANCHOR: add_poll
        var poll = channel.AddPoll(association, Request.ClassRequest(false, true, true, true), TimeSpan.FromSeconds(5));
        // ANCHOR_END: add_poll

        // start communications
        channel.Enable();

        while (true)
        {
            var input = Console.ReadLine();

            if (input == "x")
            {
                return;
            }

            try
            {
                RunOneCommand(channel, association, poll, input).GetAwaiter().GetResult();
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error: {ex}");
            }
        }
    }

    private static void RunChannel(MasterChannel channel)
    {
        // ANCHOR: association_create
        var association = channel.AddAssociation(
            1024,
            GetAssociationConfig(),
            new TestReadHandler(),
            new TestAssociationHandler(),
            new TestAssociationInformation()
        );
        // ANCHOR_END: association_create

        RunAssociation(channel, association);
    }

    private static async Task RunOneCommand(MasterChannel channel, AssociationId association, PollId poll, String input)
    {
        switch (input)
        {
            case "enable":
                {
                    channel.Enable();
                    break;
                }
            case "disable":
                {
                    channel.Disable();
                    break;
                } 
            case "dln":
                {
                    channel.SetDecodeLevel(DecodeLevel.Nothing());
                    break;
                }
            case "dlv":
                {
                    channel.SetDecodeLevel(DecodeLevel.Nothing().WithApplication(AppDecodeLevel.ObjectValues));
                    break;
                }
            case "rao":
                {
                    var request = new Request();
                    request.AddAllObjectsHeader(Variation.Group40Var0);
                    await channel.Read(association, request);
                    break;
                }
            case "rmo":
                {
                    var request = new Request();
                    request.AddAllObjectsHeader(Variation.Group10Var0);
                    request.AddAllObjectsHeader(Variation.Group40Var0);
                    await channel.Read(association, request);
                    break;
                }
            case "cmd":
                {
                    // ANCHOR: assoc_control
                    var commands = new CommandSet();
                    commands.AddG12V1U8(3, Group12Var1.FromCode(ControlCode.FromOpType(OpType.LatchOn)));
                    await channel.Operate(association, CommandMode.SelectBeforeOperate, commands);
                    // ANCHOR_END: assoc_control
                    break;
                }
            case "evt":
                {
                    channel.DemandPoll(poll);
                    break;                                        
                }                
            case "lts":
                {
                    await channel.SynchronizeTime(association, TimeSyncMode.Lan);
                    break;                                        
                }
            case "nts":
                {
                    await channel.SynchronizeTime(association, TimeSyncMode.NonLan);                    
                    break;
                }
            case "wad":
                {
                    var request = new WriteDeadBandRequest();
                    request.AddG34v1U8(3, 5);
                    request.AddG34v3U16(4, 2.5f);
                    await channel.WriteDeadBands(association, request);
                    break;                    
                }
            case "fat":
                {
                    var request = new Request();                    
                    request.AddTimeAndInterval(0, 86400000);
                    request.AddAllObjectsHeader(Variation.Group20Var0);
                    await channel.SendAndExpectEmptyResponse(association, FunctionCode.FreezeAtTime, request);                    
                    break;
                }
            case "rda":
                {
                    // ANCHOR: read_attributes
                    var request = new Request();
                    request.AddSpecificAttribute(AttributeVariations.AllAttributesRequest, 0);
                    await channel.Read(association, request);
                    // ANCHOR_END: read_attributes
                    break;
                }
            case "wda":
                {
                    // ANCHOR: write_attribute
                    var request = new Request();
                    request.AddStringAttribute(AttributeVariations.UserAssignedLocation, 0, "Mt. Olympus");
                    await channel.SendAndExpectEmptyResponse(association, FunctionCode.Write, request);
                    // ANCHOR_END: write_attribute
                    break;
                }
            case "ral":
                {
                    var request = new Request();
                    request.AddSpecificAttribute(AttributeVariations.ListOfVariations, 0);
                    await channel.Read(association, request);
                    break;
                }
            case "rd":
                {
                    // ANCHOR: read_directory
                    var items = await channel.ReadDirectory(association, ".", DirReadConfig.Defaults());
                    foreach(var info in items)
                    {
                        PrintFileInfo(info);
                    }
                    // ANCHOR_END: read_directory
                    break;
                }
            case "gfi":
                {
                    // ANCHOR: get_file_info
                    var info = await channel.GetFileInfo(association, ".");
                    PrintFileInfo(info);
                    // ANCHOR_END: get_file_info
                    break;
                }
            case "rf":
                {
                    // ANCHOR: read_file
                    channel.ReadFile(association, ".", FileReadConfig.Defaults(), new FileReader());
                    // ANCHOR_END: read_file
                    break;
                }
            case "wf":
                {
                    // ANCHOR: write_file
                    var file = await channel.OpenFile(association, "hello_world.txt", 0, Permissions.None(), 0xFFFFFFFF, FileMode.Write, 1024);
                    await channel.WriteFileBlock(association, file.FileHandle, 0, false, System.Text.Encoding.UTF8.GetBytes("hello world\n"));
                    await channel.WriteFileBlock(association, file.FileHandle, 1, true, System.Text.Encoding.UTF8.GetBytes("hello world\n"));
                    await channel.CloseFile(association, file.FileHandle);
                    // ANCHOR_END: write_file
                    break;
                }
            case "crt":
                {
                    var delay = await channel.ColdRestart(association);
                    Console.WriteLine($"Restart delay: {delay}");
                    break;
                }
            case "wrt":
                {
                    var delay = await channel.WarmRestart(association);
                    Console.WriteLine($"Restart delay: {delay}");
                    break;
                }
            case "lsr":
                {
                    var result = await channel.CheckLinkStatus(association);
                    Console.WriteLine($"Result: {result}");
                    break;
                }
            default:
                Console.WriteLine("Unknown command");
                break;
        }
    }
    private static void PrintFileInfo(FileInfo info)
    {
        Console.WriteLine($"Filename: {info.FileName}");
        Console.WriteLine($"  type: {info.FileType}");
        Console.WriteLine($"  size: {info.Size}");
        Console.WriteLine($"  created: {info.TimeCreated}");
    }
}
