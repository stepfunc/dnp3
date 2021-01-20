using System;
using System.Threading.Tasks;
using System.Collections.Generic;
using dnp3rs;

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

        public RestartDelay ColdRestart()
        {
            return RestartDelay.ValidSeconds(60);
        }

        public RestartDelay WarmRestart()
        {
            return RestartDelay.NotSupported();
        }
    }

    class TestOutstationInformation : IOutstationInformation
    {
        public void ProcessRequestFromIdle(RequestHeader header) { }

        public void BroadcastReceived(FunctionCode functionCode, BroadcastAction action) { }

        public void EnterSolicitedConfirmWait(byte ecsn) { }

        public void SolicitedConfirmTimeout(byte ecsn) { }

        public void SolicitedConfirmReceived(byte ecsn) { }

        public void SolicitedConfirmWaitNewRequest(RequestHeader header) { }

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

        public CommandStatus SelectG12v1(G12v1 control, ushort index, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG12v1(G12v1 control, ushort index, OperateType opType, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus SelectG41v1(int control, ushort index, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG41v1(int control, ushort index, OperateType opType, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus SelectG41v2(short value, ushort index, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG41v2(short value, ushort index, OperateType opType, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus SelectG41v3(float value, ushort index, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG41v3(float value, ushort index, OperateType opType, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus SelectG41v4(double value, ushort index, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG41v4(double value, ushort index, OperateType opType, Database database) { return CommandStatus.NotSupported; }
    }

    class OutstationTransaction : IOutstationTransaction
    {
        readonly Action<Database> action;

        public OutstationTransaction(Action<Database> action)
        {
            this.action = action;
        }

        public void Execute(Database database)
        {
            this.action.Invoke(database);
        }
    }

    public static void Main(string[] args)
    {   
            MainAsync().GetAwaiter().GetResult();
    }

    private static async Task MainAsync()
    {        
        Logging.Configure(
            new LoggingConfiguration { Level = LogLevel.Info, PrintLevel = true, PrintModuleInfo = false, TimeFormat = TimeFormat.System, OutputFormat = LogOutputFormat.Text },
            new TestLogger()
        );

        using (var runtime = new Runtime(new RuntimeConfig { NumCoreThreads = 4 }))
        {
            using (var server = new TcpServer(runtime, "127.0.0.1:20000"))
            {
                var config = OutstationConfig.DefaultConfig(1024, 1);
                config.LogLevel = DecodeLogLevel.ObjectValues;
                var database = DatabaseConfig.DefaultConfig();
                database.Events.MaxAnalog = 10;
                var application = new TestOutstationApplication();
                var information = new TestOutstationInformation();
                var controlHandler = new TestControlHandler();
                var addressFilter = AddressFilter.Any();
                var outstation = server.AddOutstation(config, database, application, information, controlHandler, addressFilter);

                outstation.Transaction(new OutstationTransaction((db) =>
                {
                    for(int i = 0; i < 10; i++)
                    {
                        db.AddAnalog((ushort)i, EventClass.Class1, new AnalogConfig
                        {
                            StaticVariation = StaticAnalogVariation.Group30Var1,
                            EventVariation = EventAnalogVariation.Group32Var1,
                            Deadband = 0.0,
                        });

                        db.UpdateAnalog(new Analog
                        {
                            Index = (ushort)i,
                            Value = 10.0,
                            Flags = new Flags { Value = 0x00 },
                            Time = new Timestamp
                            {
                                Quality = TimeQuality.Synchronized,
                                Value = 0
                            },
                        }, UpdateOptions.DefaultOptions());
                    }
                }));

                server.Bind();

                var value = 0.0;

                while (true)
                {
                    switch (await GetInputAsync())
                    {
                        case "x":
                            return;
                        case "b":
                            {
                                outstation.Transaction(new OutstationTransaction((db) =>
                                {
                                    db.UpdateAnalog(new Analog
                                    {
                                        Index = 7,
                                        Value = value,
                                        Flags = new Flags { Value = 0x00 },
                                        Time = new Timestamp
                                        {
                                            Quality = TimeQuality.Synchronized,
                                            Value = 0
                                        },
                                    }, UpdateOptions.DefaultOptions());
                                }));
                                value++;
                                break;
                            }
                        default:
                            Console.WriteLine("Unknown command");
                            break;
                    }
                }
            }
        }
    }

    private static Task<string> GetInputAsync()
    {
        return Task.Run(() => Console.ReadLine());
    }
}
