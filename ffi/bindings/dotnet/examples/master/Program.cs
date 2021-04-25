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
            Console.WriteLine($"Beginning fragment (broadcast: {header.Iin.Iin1.IsSet(Iin1Flag.Broadcast)})");
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
        public TimestampUtc GetCurrentTime()
        {
            return TimestampUtc.Valid((ulong)DateTime.UtcNow.Subtract(DateTime.UnixEpoch).TotalMilliseconds);
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


        using (
        // ANCHOR: runtime_init
        var runtime = new Runtime(new RuntimeConfig { NumCoreThreads = 4 })
        // ANCHOR_END: runtime_init
        )
        {
            using (
            // ANCHOR: create_master_channel
            var channel = MasterChannel.CreateTcpChannel(
                runtime, LinkErrorMode.Close,
                GetMasterChannelConfig(),
                new EndpointList("127.0.0.1:20000"),
                new RetryStrategy(),
                TimeSpan.FromSeconds(1), 
                new TestListener())
            // ANCHOR_END: create_master_channel
            )
            {
                RunChannel(channel).GetAwaiter().GetResult();
            }
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

    private static async Task RunChannel(MasterChannel channel)
    {
        var association = channel.AddAssociation(
            1024,
            GetAssociationConfig(),
            new TestReadHandler(),
            new TestAssocationHandler()
        );

        var poll = channel.AddPoll(association, Request.ClassRequest(false, true, true, true), TimeSpan.FromSeconds(5));

        // start communications
        channel.Enable();

        while (true)
        {
            switch (await GetInputAsync())
            {
                case "x":
                    return;
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
                        channel.SetDecodeLevel(new DecodeLevel());
                        break;
                    }
                case "dlv":
                    {
                        channel.SetDecodeLevel(new DecodeLevel() { Application = AppDecodeLevel.ObjectValues });
                        break;
                    }
                case "rao":
                    {
                        var request = new Request();
                        request.AddAllObjectsHeader(Variation.Group40Var0);
                        var result = await channel.Read(association, request);
                        Console.WriteLine($"Result: {result}");
                        break;
                    }
                case "rmo":
                    {
                        var request = new Request();
                        request.AddAllObjectsHeader(Variation.Group10Var0);
                        request.AddAllObjectsHeader(Variation.Group40Var0);
                        var result = await channel.Read(association, request);
                        Console.WriteLine($"Result: {result}");
                        break;
                    }
                case "cmd":
                    {
                        var commands = new Commands();
                        commands.AddG12v1u8(3, new G12v1(new ControlCode(TripCloseCode.Nul, false, OpType.LatchOn), 1, 1000, 1000));
                        var result = await channel.Operate(association, CommandMode.SelectBeforeOperate, commands);
                        Console.WriteLine($"Result: {result}");
                        break;
                    }
                case "evt":
                    {
                        channel.DemandPoll(poll);
                        break;
                    }
                case "lts":
                    {
                        var result = await channel.SynchronizeTime(association, TimeSyncMode.Lan);
                        Console.WriteLine($"Result: {result}");
                        break;
                    }
                case "nts":
                    {
                        var result = await channel.SynchronizeTime(association, TimeSyncMode.NonLan);
                        Console.WriteLine($"Result: {result}");
                        break;
                    }
                case "crt":
                    {
                        var result = await channel.ColdRestart(association);
                        Console.WriteLine($"Result: {result}");
                        break;
                    }
                case "wrt":
                    {
                        var result = await channel.WarmRestart(association);
                        Console.WriteLine($"Result: {result}");
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
    }

    private static Task<string> GetInputAsync()
    {
        return Task.Run(() => Console.ReadLine());
    }
}
