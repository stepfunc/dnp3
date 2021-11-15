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

    class TestListener : IClientStateListener
    {
        public void OnChange(ClientState state)
        {
            Console.WriteLine(state);
        }
    }

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

        public void HandleBinary(HeaderInfo info, ICollection<Binary> values)
        {
            Console.WriteLine("Binaries:");
            Console.WriteLine("Qualifier: " + info.Qualifier);
            Console.WriteLine("Variation: " + info.Variation);

            foreach (var val in values)
            {
                Console.WriteLine($"BI {val.Index}: Value={val.Value} Flags={val.Flags.Value} Time={val.Time.Value} ({val.Time.Quality})");
            }
        }

        public void HandleDoubleBitBinary(HeaderInfo info, ICollection<DoubleBitBinary> values)
        {
            Console.WriteLine("Double Bit Binaries:");
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

        public void HandleAnalog(HeaderInfo info, ICollection<Analog> values)
        {
            Console.WriteLine("Analogs:");
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

        public void HandleOctetString(HeaderInfo info, ICollection<OctetString> values)
        {
            Console.WriteLine("Octet String:");
            Console.WriteLine("Qualifier: " + info.Qualifier);
            Console.WriteLine("Variation: " + info.Variation);

            foreach (var val in values)
            {
                Console.Write($"Octet String {val.Index}: Value=");
                foreach (var b in val.Value)
                {
                    Console.Write($"{b.Value:X2} ");
                }
                Console.WriteLine();
            }
        }
    }

    // ANCHOR: association_handler
    class TestAssocationHandler : IAssociationHandler
    {
        public UtcTimestamp GetCurrentTime()
        {
            return UtcTimestamp.Valid((ulong)DateTime.UtcNow.Subtract(DateTime.UnixEpoch).TotalMilliseconds);
        }
    }
    // ANCHOR_END: association_handler

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

        try
        {
            Run(runtime);
        }
        finally
        {
            // ANCHOR: runtime_shutdown
            runtime.Shutdown();
            // ANCHOR_END: runtime_shutdown
        }
    }

    // ANCHOR: master_channel_config
    private static MasterChannelConfig GetMasterChannelConfig()
    {
        var config = new MasterChannelConfig(1);
        config.DecodeLevel.Application = AppDecodeLevel.ObjectValues;
        return config;
    }
    // ANCHOR_END: master_channel_config

    // ANCHOR: association_config
    private static AssociationConfig GetAssociationConfig()
    {
        var config = new AssociationConfig(
            // disable unsolicited first (Class 1/2/3)
            EventClasses.All(),
            // after the integrity poll, enable unsolicited (Class 1/2/3)
            EventClasses.All(),
            // perform startup integrity poll with Class 1/2/3/0
            Classes.All(),
            // don't automatically scan Class 1/2/3 when the corresponding IIN bit is asserted
            EventClasses.None()
        );
        config.AutoTimeSync = AutoTimeSync.Lan;
        config.KeepAliveTimeout = TimeSpan.FromSeconds(60);
        return config;
    }
    // ANCHOR_END: association_config

    private static void Run(Runtime runtime)
    {
        // ANCHOR: create_master_channel
        var channel = MasterChannel.CreateTcpChannel(
            runtime,
            LinkErrorMode.Close,
            GetMasterChannelConfig(),
            new EndpointList("127.0.0.1:20000"),
            new ConnectStrategy(),
            new TestListener()
        );
        // ANCHOR_END: create_master_channel

        // ANCHOR: association_create
        var association = channel.AddAssociation(
            1024,
            GetAssociationConfig(),
            new TestReadHandler(),
            new TestAssocationHandler()
        );
        // ANCHOR_END: association_create

        // ANCHOR: add_poll
        var poll = channel.AddPoll(association, Request.ClassRequest(false, true, true, true), TimeSpan.FromSeconds(5));
        // ANCHOR_END: add_poll

        // start communications
        channel.Enable();

        while (true)
        {
            var input = Console.ReadLine();
            try
            {
                if (!RunOneCommand(channel, association, poll, input).GetAwaiter().GetResult())
                {
                    // exit command
                    return;
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine("Error: " + ex);
            }
        }
    }

    private static async Task<bool> RunOneCommand(MasterChannel channel, AssociationId association, PollId poll, String input)
    {
        switch (input)
        {
            case "x":
                return false;
            case "enable":
                {
                    channel.Enable();
                    return true;
                }
            case "disable":
                {
                    channel.Disable();
                    return true;
                }
            case "dln":
                {
                    channel.SetDecodeLevel(new DecodeLevel());
                    return true;
                }
            case "dlv":
                {
                    channel.SetDecodeLevel(new DecodeLevel() { Application = AppDecodeLevel.ObjectValues });
                    return true;
                }
            case "rao":
                {
                    var request = new Request();
                    request.AddAllObjectsHeader(Variation.Group40Var0);
                    var result = await channel.Read(association, request);
                    Console.WriteLine($"Result: {result}");
                    return true;
                }
            case "rmo":
                {
                    var request = new Request();
                    request.AddAllObjectsHeader(Variation.Group10Var0);
                    request.AddAllObjectsHeader(Variation.Group40Var0);
                    var result = await channel.Read(association, request);
                    Console.WriteLine($"Result: {result}");
                    return true;
                }
            case "cmd":
                {
                    // ANCHOR: assoc_control
                    var commands = new CommandSet();
                    commands.AddG12V1U8(3, new Group12Var1(new ControlCode(TripCloseCode.Nul, false, OpType.LatchOn), 1, 1000, 1000));
                    var result = await channel.Operate(association, CommandMode.SelectBeforeOperate, commands);
                    Console.WriteLine($"Result: {result}");
                    // ANCHOR_END: assoc_control
                    return true;
                }
            case "evt":
                {
                    channel.DemandPoll(poll);
                    return true;
                }
            case "lts":
                {
                    await channel.SynchronizeTime(association, TimeSyncMode.Lan);
                    Console.WriteLine("Time sync success");
                    return true;
                }
            case "nts":
                {
                    await channel.SynchronizeTime(association, TimeSyncMode.NonLan);
                    Console.WriteLine("Time sync success");
                    return true;
                }
            case "crt":
                {
                    var delay = await channel.ColdRestart(association);
                    Console.WriteLine($"Restart delay: {delay}");
                    return true;
                }
            case "wrt":
                {
                    var delay = await channel.WarmRestart(association);
                    Console.WriteLine($"Restart delay: {delay}");
                    return true;
                }
            case "lsr":
                {
                    var result = await channel.CheckLinkStatus(association);
                    Console.WriteLine($"Result: {result}");
                    return true;
                }
            default:
                Console.WriteLine("Unknown command");
                return true;
        }
    }
}
