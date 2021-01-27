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
                // ANCHOR: outstation_config
                // create an outstation configuration with default values
                var config = OutstationConfig.DefaultConfig(
                    // outstation address
                    1024,
                    // master address
                    1
                );
                // override the default decode log level
                config.LogLevel = DecodeLogLevel.ObjectValues;
                // ANCHOR_END: outstation_config

                var database = DatabaseConfig.DefaultConfig();
                database.Events = EventBufferConfig.AllTypes(10);
                var application = new TestOutstationApplication();
                var information = new TestOutstationInformation();
                var controlHandler = new TestControlHandler();
                var addressFilter = AddressFilter.Any();
                var outstation = server.AddOutstation(config, database, application, information, controlHandler, addressFilter);

                // Setup initial points
                outstation.Transaction(new OutstationTransaction((db) =>
                {
                    for(ushort i = 0; i < 10; i++)
                    {
                        db.AddBinary(i, EventClass.Class1, new BinaryConfig
                        {
                            StaticVariation = StaticBinaryVariation.Group1Var2,
                            EventVariation = EventBinaryVariation.Group2Var2,
                        });
                        db.AddDoubleBitBinary(i, EventClass.Class1, new DoubleBitBinaryConfig
                        {
                            StaticVariation = StaticDoubleBitBinaryVariation.Group3Var2,
                            EventVariation = EventDoubleBitBinaryVariation.Group4Var2,
                        });
                        db.AddBinaryOutputStatus(i, EventClass.Class1, new BinaryOutputStatusConfig
                        {
                            StaticVariation = StaticBinaryOutputStatusVariation.Group10Var2,
                            EventVariation = EventBinaryOutputStatusVariation.Group11Var2,
                        });
                        db.AddCounter(i, EventClass.Class1, new CounterConfig
                        {
                            StaticVariation = StaticCounterVariation.Group20Var1,
                            EventVariation = EventCounterVariation.Group22Var1,
                            Deadband = 0,
                        });
                        db.AddFrozenCounter(i, EventClass.Class1, new FrozenCounterConfig
                        {
                            StaticVariation = StaticFrozenCounterVariation.Group21Var5,
                            EventVariation = EventFrozenCounterVariation.Group23Var5,
                            Deadband = 0,
                        });
                        db.AddAnalog(i, EventClass.Class1, new AnalogConfig
                        {
                            StaticVariation = StaticAnalogVariation.Group30Var6,
                            EventVariation = EventAnalogVariation.Group32Var8,
                            Deadband = 0.0,
                        });
                        db.AddAnalogOutputStatus(i, EventClass.Class1, new AnalogOutputStatusConfig
                        {
                            StaticVariation = StaticAnalogOutputStatusVariation.Group40Var4,
                            EventVariation = EventAnalogOutputStatusVariation.Group42Var8,
                            Deadband = 0.0,
                        });
                        db.AddOctetString(i, EventClass.Class1);

                        var flags = new Flags { Value = 0x00 }.Set(Flag.Restart, true);
                        db.UpdateBinary(new Binary
                        {
                            Index = i,
                            Value = false,
                            Flags = flags,
                            Time = Timestamp.InvalidTimestamp(),
                        }, UpdateOptions.DefaultOptions());
                        db.UpdateDoubleBitBinary(new DoubleBitBinary
                        {
                            Index = i,
                            Value = DoubleBit.Indeterminate,
                            Flags = flags,
                            Time = Timestamp.InvalidTimestamp(),
                        }, UpdateOptions.DefaultOptions());
                        db.UpdateBinaryOutputStatus(new BinaryOutputStatus
                        {
                            Index = i,
                            Value = false,
                            Flags = flags,
                            Time = Timestamp.InvalidTimestamp(),
                        }, UpdateOptions.DefaultOptions());
                        db.UpdateCounter(new Counter
                        {
                            Index = i,
                            Value = 0,
                            Flags = flags,
                            Time = Timestamp.InvalidTimestamp(),
                        }, UpdateOptions.DefaultOptions());
                        db.UpdateFrozenCounter(new FrozenCounter
                        {
                            Index = i,
                            Value = 0,
                            Flags = flags,
                            Time = Timestamp.InvalidTimestamp(),
                        }, UpdateOptions.DefaultOptions());
                        db.UpdateAnalog(new Analog
                        {
                            Index = i,
                            Value = 0.0,
                            Flags = flags,
                            Time = Timestamp.InvalidTimestamp(),
                        }, UpdateOptions.DefaultOptions());
                        db.UpdateAnalogOutputStatus(new AnalogOutputStatus
                        {
                            Index = i,
                            Value = 0.0,
                            Flags = flags,
                            Time = Timestamp.InvalidTimestamp(),
                        }, UpdateOptions.DefaultOptions());
                    }
                }));

                // Start the outstation
                server.Bind();

                var binaryValue = false;
                var doubleBitBinaryValue = DoubleBit.DeterminedOff;
                var binaryOutputStatusValue = false;
                var counterValue = (uint)0;
                var frozenCounterValue = (uint)0;
                var analogValue = 0.0;
                var analogOutputStatusValue = 0.0;

                while (true)
                {
                    switch (await GetInputAsync())
                    {
                        case "x":
                            return;
                        case "bi":
                            {
                                outstation.Transaction(new OutstationTransaction((db) =>
                                {
                                    binaryValue = !binaryValue;
                                    db.UpdateBinary(new Binary
                                    {
                                        Index = 7,
                                        Value = binaryValue,
                                        Flags = new Flags { Value = 0x00 }.Set(Flag.Online, true),
                                        Time = Timestamp.SynchronizedTimestamp(0),
                                    }, UpdateOptions.DefaultOptions());
                                }));
                                break;
                            }
                        case "dbbi":
                            {
                                outstation.Transaction(new OutstationTransaction((db) =>
                                {
                                    doubleBitBinaryValue = doubleBitBinaryValue == DoubleBit.DeterminedOff ? DoubleBit.DeterminedOn : DoubleBit.DeterminedOff;
                                    db.UpdateDoubleBitBinary(new DoubleBitBinary
                                    {
                                        Index = 7,
                                        Value = doubleBitBinaryValue,
                                        Flags = new Flags { Value = 0x00 }.Set(Flag.Online, true),
                                        Time = Timestamp.SynchronizedTimestamp(0),
                                    }, UpdateOptions.DefaultOptions());
                                }));
                                break;
                            }
                        case "bos":
                            {
                                outstation.Transaction(new OutstationTransaction((db) =>
                                {
                                    binaryOutputStatusValue = !binaryOutputStatusValue;
                                    db.UpdateBinaryOutputStatus(new BinaryOutputStatus
                                    {
                                        Index = 7,
                                        Value = binaryOutputStatusValue,
                                        Flags = new Flags { Value = 0x00 }.Set(Flag.Online, true),
                                        Time = Timestamp.SynchronizedTimestamp(0),
                                    }, UpdateOptions.DefaultOptions());
                                }));
                                break;
                            }
                        case "co":
                            {
                                outstation.Transaction(new OutstationTransaction((db) =>
                                {
                                    db.UpdateCounter(new Counter
                                    {
                                        Index = 7,
                                        Value = ++counterValue,
                                        Flags = new Flags { Value = 0x00 }.Set(Flag.Online, true),
                                        Time = Timestamp.SynchronizedTimestamp(0),
                                    }, UpdateOptions.DefaultOptions());
                                }));
                                break;
                            }
                        case "fco":
                            {
                                outstation.Transaction(new OutstationTransaction((db) =>
                                {
                                    db.UpdateFrozenCounter(new FrozenCounter
                                    {
                                        Index = 7,
                                        Value = ++frozenCounterValue,
                                        Flags = new Flags { Value = 0x00 }.Set(Flag.Online, true),
                                        Time = Timestamp.SynchronizedTimestamp(0),
                                    }, UpdateOptions.DefaultOptions());
                                }));
                                break;
                            }
                        case "ai":
                            {
                                outstation.Transaction(new OutstationTransaction((db) =>
                                {
                                    db.UpdateAnalog(new Analog
                                    {
                                        Index = 7,
                                        Value = ++analogValue,
                                        Flags = new Flags { Value = 0x00 }.Set(Flag.Online, true),
                                        Time = Timestamp.SynchronizedTimestamp(0),
                                    }, UpdateOptions.DefaultOptions());
                                }));
                                break;
                            }
                        case "aos":
                            {
                                outstation.Transaction(new OutstationTransaction((db) =>
                                {
                                    db.UpdateAnalogOutputStatus(new AnalogOutputStatus
                                    {
                                        Index = 7,
                                        Value = ++analogOutputStatusValue,
                                        Flags = new Flags { Value = 0x00 }.Set(Flag.Online, true),
                                        Time = Timestamp.SynchronizedTimestamp(0),
                                    }, UpdateOptions.DefaultOptions());
                                }));
                                break;
                            }
                        case "os":
                            {
                                outstation.Transaction(new OutstationTransaction((db) =>
                                {
                                    db.UpdateOctetString(7, System.Text.Encoding.ASCII.GetBytes("Hello"), UpdateOptions.DefaultOptions());
                                }));
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
