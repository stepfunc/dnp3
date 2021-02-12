using System;
using System.Threading.Tasks;
using System.Collections.Generic;
using dnp3rs;

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
        public void BeginFragment(ResponseHeader header)
        {
            Console.WriteLine($"Beginning fragment (broadcast: {header.Iin.Iin1.IsSet(Iin1Flag.Broadcast)})");
        }

        public void EndFragment(ResponseHeader header)
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

    class TestTimeProvider : ITimeProvider
    {
        public TimeProviderTimestamp GetTime()
        {
            return TimeProviderTimestamp.Valid((ulong)DateTime.UtcNow.Subtract(DateTime.UnixEpoch).TotalMilliseconds);
        }
    }

    public static void Main(string[] args)
    {
        // ANCHOR: logging_init
        // called once during program initialization
        Logging.Configure(
            new LoggingConfiguration
            {
                Level = LogLevel.Info,
                PrintLevel = true,
                PrintModuleInfo = false,
                TimeFormat = TimeFormat.System,
                OutputFormat = LogOutputFormat.Text
            },
            new ConsoleLogger()
        );
        // ANCHOR_END: logging_init

        // ANCHOR: runtime_init
        using (var runtime = new Runtime(new RuntimeConfig { NumCoreThreads = 4 }))
        // ANCHOR_END: runtime_init
        {
            MainAsync(runtime).GetAwaiter().GetResult();
        }
    }

    private static MasterConfig GetMasterConfig()
    {
        // create a default configuration with a master address of "1"
        var config = new MasterConfig(1)
        {
            // override the reconnect strategy            
            ReconnectionStrategy = new RetryStrategy
            {
                MinDelay = TimeSpan.FromMilliseconds(100),
                MaxDelay = TimeSpan.FromSeconds(5),
            }
        };
        
        config.DecodeLevel.Application = AppDecodeLevel.ObjectValues;

        return config;
    }

    private static async Task MainAsync(Runtime runtime)
    {

        var master = Master.CreateTcpSession(
            runtime,
            LinkErrorMode.Close,
            GetMasterConfig(),
            new EndpointList("127.0.0.1:20000"),
            new TestListener()
        );

        var readHandler = new TestReadHandler();
        var association = master.AddAssociation(
            1024,
            new AssociationConfig(new EventClasses(true, true, true), new EventClasses(true, true, true), Classes.All(), new EventClasses(false, false, false))
            {
                AutoTimeSync = AutoTimeSync.Lan,
                AutoTasksRetryStrategy = new RetryStrategy
                {
                    MinDelay = TimeSpan.FromSeconds(1),
                    MaxDelay = TimeSpan.FromSeconds(5),
                },
                KeepAliveTimeout = TimeSpan.FromSeconds(60),
            },
            new AssociationHandlers(readHandler, readHandler, readHandler),
            new TestTimeProvider()
        );

        var pollRequest = Request.ClassRequest(false, true, true, true);
        var poll = association.AddPoll(pollRequest, TimeSpan.FromSeconds(5));

        while (true)
        {
            switch (await GetInputAsync())
            {
                case "x":
                    return;
                case "dln":
                    {
                        master.SetDecodeLevel(new DecodeLevel());
                        break;
                    }
                case "dlv":
                    {                        
                        master.SetDecodeLevel(new DecodeLevel() { Application = AppDecodeLevel.ObjectValues });
                        break;
                    }
                case "rao":
                    {
                        var request = new Request();
                        request.AddAllObjectsHeader(Variation.Group40Var0);
                        var result = await association.Read(request);
                        Console.WriteLine($"Result: {result}");
                        break;
                    }
                case "rmo":
                    {
                        var request = new Request();
                        request.AddAllObjectsHeader(Variation.Group10Var0);
                        request.AddAllObjectsHeader(Variation.Group40Var0);
                        var result = await association.Read(request);
                        Console.WriteLine($"Result: {result}");
                        break;
                    }
                case "cmd":
                    {
                        var command = new Command();
                        command.AddU16g12v1(3, new G12v1(new ControlCode(TripCloseCode.Nul, false, OpType.LatchOn), 1, 1000, 1000));
                        var result = await association.Operate(CommandMode.SelectBeforeOperate, command);
                        Console.WriteLine($"Result: {result}");
                        break;
                    }
                case "evt":
                    {
                        poll.Demand();
                        break;
                    }
                case "lts":
                    {
                        var result = await association.PerformTimeSync(TimeSyncMode.Lan);
                        Console.WriteLine($"Result: {result}");
                        break;
                    }
                case "nts":
                    {
                        var result = await association.PerformTimeSync(TimeSyncMode.NonLan);
                        Console.WriteLine($"Result: {result}");
                        break;
                    }
                case "crt":
                    {
                        var result = await association.ColdRestart();
                        Console.WriteLine($"Result: {result}");
                        break;
                    }
                case "wrt":
                    {
                        var result = await association.WarmRestart();
                        Console.WriteLine($"Result: {result}");
                        break;
                    }
                case "lsr":
                    {
                        var result = await association.CheckLinkStatus();
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
