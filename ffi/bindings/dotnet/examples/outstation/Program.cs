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

        public ApplicationIin GetApplicationIin()
        {
            return new ApplicationIin();
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

    public static void Main(string[] args)
    {   
            MainAsync().GetAwaiter().GetResult();
    }

    private static async Task MainAsync()
    {
        // Initialize logging with the default configuration
        Logging.Configure(
            new LoggingConfig(),
            new TestLogger()
        );

        using (var runtime = new Runtime(new RuntimeConfig { NumCoreThreads = 4 }))
        {
            using (var server = new TcpServer(runtime, LinkErrorMode.Close, "127.0.0.1:20000"))
            {
                // ANCHOR: outstation_config
                // create an outstation configuration with default values
                var config = new OutstationConfig(
                    // outstation address
                    1024,
                    // master address
                    1
                );
                // override the default application decoding level
                config.DecodeLevel.Application = AppDecodeLevel.ObjectValues;
                // ANCHOR_END: outstation_config
                                
                var application = new TestOutstationApplication();
                var information = new TestOutstationInformation();
                var controlHandler = new TestControlHandler();
                var addressFilter = AddressFilter.Any();
                var outstation = server.AddOutstation(config, EventBufferConfig.AllTypes(10), application, information, controlHandler, addressFilter);

                // Setup initial points
                outstation.Transaction(new OutstationTransaction((db) =>
                {
                    for(ushort i = 0; i < 10; i++)
                    {
                        db.AddBinary(i, EventClass.Class1, new BinaryConfig());
                        db.AddDoubleBitBinary(i, EventClass.Class1, new DoubleBitBinaryConfig());
                        db.AddBinaryOutputStatus(i, EventClass.Class1, new BinaryOutputStatusConfig());
                        db.AddCounter(i, EventClass.Class1, new CounterConfig());
                        db.AddFrozenCounter(i, EventClass.Class1, new FrozenCounterConfig());
                        db.AddAnalog(i, EventClass.Class1, new AnalogConfig());
                        db.AddAnalogOutputStatus(i, EventClass.Class1, new AnalogOutputStatusConfig());
                        db.AddOctetString(i, EventClass.Class1);

                        var restart = new Flags(Flag.Restart);
                        db.UpdateBinary(new Binary(i, false, restart, Timestamp.InvalidTimestamp()), new UpdateOptions());
                        db.UpdateDoubleBitBinary(new DoubleBitBinary(i, DoubleBit.Indeterminate, restart, Timestamp.InvalidTimestamp()), new UpdateOptions());
                        db.UpdateBinaryOutputStatus(new BinaryOutputStatus(i, false, restart, Timestamp.InvalidTimestamp()), new UpdateOptions());
                        db.UpdateCounter(new Counter(i, 0, restart, Timestamp.InvalidTimestamp()), new UpdateOptions());
                        db.UpdateFrozenCounter(new FrozenCounter(i, 0, restart, Timestamp.InvalidTimestamp()), new UpdateOptions());
                        db.UpdateAnalog(new Analog(i, 0.0, restart, Timestamp.InvalidTimestamp()), new UpdateOptions());
                        db.UpdateAnalogOutputStatus(new AnalogOutputStatus(i, 0.0, restart, Timestamp.InvalidTimestamp()), new UpdateOptions());
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
                                    db.UpdateBinary(new Binary(7, binaryValue, new Flags(Flag.Online), Timestamp.SynchronizedTimestamp(0)), new UpdateOptions());
                                }));
                                break;
                            }
                        case "dbbi":
                            {
                                outstation.Transaction(new OutstationTransaction((db) =>
                                {
                                    doubleBitBinaryValue = doubleBitBinaryValue == DoubleBit.DeterminedOff ? DoubleBit.DeterminedOn : DoubleBit.DeterminedOff;
                                    db.UpdateDoubleBitBinary(new DoubleBitBinary(7, doubleBitBinaryValue, new Flags(Flag.Online), Timestamp.SynchronizedTimestamp(0)), new UpdateOptions());
                                }));
                                break;
                            }
                        case "bos":
                            {
                                outstation.Transaction(new OutstationTransaction((db) =>
                                {
                                    binaryOutputStatusValue = !binaryOutputStatusValue;
                                    db.UpdateBinaryOutputStatus(new BinaryOutputStatus(7, binaryOutputStatusValue, new Flags(Flag.Online), Timestamp.SynchronizedTimestamp(0)), new UpdateOptions());
                                }));
                                break;
                            }
                        case "co":
                            {
                                outstation.Transaction(new OutstationTransaction((db) =>
                                {
                                    db.UpdateCounter(new Counter(7, ++counterValue, new Flags(Flag.Online), Timestamp.SynchronizedTimestamp(0)), new UpdateOptions());
                                }));
                                break;
                            }
                        case "fco":
                            {
                                outstation.Transaction(new OutstationTransaction((db) =>
                                {
                                    db.UpdateFrozenCounter(new FrozenCounter(7, ++frozenCounterValue, new Flags(Flag.Online), Timestamp.SynchronizedTimestamp(0)), new UpdateOptions());
                                }));
                                break;
                            }
                        case "ai":
                            {
                                outstation.Transaction(new OutstationTransaction((db) =>
                                {
                                    db.UpdateAnalog(new Analog(7, ++analogValue, new Flags(Flag.Online), Timestamp.SynchronizedTimestamp(0)), new UpdateOptions());
                                }));
                                break;
                            }
                        case "aos":
                            {
                                outstation.Transaction(new OutstationTransaction((db) =>
                                {
                                    db.UpdateAnalogOutputStatus(new AnalogOutputStatus(7, ++analogOutputStatusValue, new Flags(Flag.Online), Timestamp.SynchronizedTimestamp(0)), new UpdateOptions());
                                }));
                                break;
                            }
                        case "os":
                            {
                                outstation.Transaction(new OutstationTransaction((db) =>
                                {
                                    db.UpdateOctetString(7, System.Text.Encoding.ASCII.GetBytes("Hello"), new UpdateOptions());
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
