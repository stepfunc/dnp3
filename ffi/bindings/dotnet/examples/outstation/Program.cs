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

        public FreezeResult FreezeCountersAll(FreezeType freezeType, Database database) { return FreezeResult.NotSupported; }

        public FreezeResult FreezeCountersRange(ushort start, ushort stop, FreezeType freezeType, Database database) { return FreezeResult.NotSupported; }
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

        public CommandStatus SelectG12v1(Group12Var1 control, ushort index, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG12v1(Group12Var1 control, ushort index, OperateType opType, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus SelectG41v1(int control, ushort index, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG41v1(int control, ushort index, OperateType opType, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus SelectG41v2(short value, ushort index, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG41v2(short value, ushort index, OperateType opType, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus SelectG41v3(float value, ushort index, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG41v3(float value, ushort index, OperateType opType, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus SelectG41v4(double value, ushort index, Database database) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG41v4(double value, ushort index, OperateType opType, Database database) { return CommandStatus.NotSupported; }
    }

    class TestConnectionStateListener : IConnectionStateListener
    {
        public void OnChange(ConnectionState state)
        {
            Console.WriteLine("Connection state change: " + state);
        }
    }

    public static void Main(string[] args)
    {
        // Initialize logging with the default configuration
        Logging.Configure(
            new LoggingConfig(),
            new TestLogger()
        );

        var runtime = new Runtime(new RuntimeConfig { NumCoreThreads = 4 });
        // ANCHOR: create_tcp_server
        var server = new TcpServer(runtime, LinkErrorMode.Close, "127.0.0.1:20000");
        // ANCHOR_END: create_tcp_server

        try
        {
            RunServer(server);
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
        return Timestamp.SynchronizedTimestamp((ulong) DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
    }

    private static void RunServer(TcpServer server)
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

        // ANCHOR: tcp_server_add_outstation
        var outstation = server.AddOutstation(
            config,
            GetEventBufferConfig(),
            new TestOutstationApplication(),
            new TestOutstationInformation(),
            new TestControlHandler(),
            new TestConnectionStateListener(),
            AddressFilter.Any()
        );
        // ANCHOR_END: tcp_server_add_outstation

        // Setup initial points
        // ANCHOR: database_init
        outstation.Transaction(db =>
        {
            for (ushort i = 0; i < 10; i++)
            {
                // add points with default values
                db.AddBinary(i, EventClass.Class1, new BinaryConfig());
                db.AddDoubleBitBinary(i, EventClass.Class1, new DoubleBitBinaryConfig());
                db.AddBinaryOutputStatus(i, EventClass.Class1, new BinaryOutputStatusConfig());
                db.AddCounter(i, EventClass.Class1, new CounterConfig());
                db.AddFrozenCounter(i, EventClass.Class1, new FrozenCounterConfig());
                db.AddAnalog(i, EventClass.Class1, new AnalogConfig());
                db.AddAnalogOutputStatus(i, EventClass.Class1, new AnalogOutputStatusConfig());
                db.AddOctetString(i, EventClass.Class1);
            }
        });
        // ANCHOR_END: database_init

        // Start the outstation
        // ANCHOR: tcp_server_bind
        server.Bind();
        // ANCHOR_END: tcp_server_bind

        var binaryValue = false;
        var doubleBitBinaryValue = DoubleBit.DeterminedOff;
        var binaryOutputStatusValue = false;
        var counterValue = (uint)0;
        var frozenCounterValue = (uint)0;
        var analogValue = 0.0;
        var analogOutputStatusValue = 0.0;

        while (true)
        {
            switch (Console.ReadLine())
            {
                case "x":
                    return;
                case "bi":
                    {
                        outstation.Transaction(db =>
                        {
                            binaryValue = !binaryValue;
                            db.UpdateBinary(new BinaryInput(7, binaryValue, new Flags(Flag.Online), Now()), new UpdateOptions());
                        });
                        break;
                    }
                case "dbbi":
                    {
                        outstation.Transaction(db =>
                        {
                            doubleBitBinaryValue = doubleBitBinaryValue == DoubleBit.DeterminedOff ? DoubleBit.DeterminedOn : DoubleBit.DeterminedOff;
                            db.UpdateDoubleBitBinary(new DoubleBitBinaryInput(7, doubleBitBinaryValue, new Flags(Flag.Online), Now()), new UpdateOptions());
                        });
                        break;
                    }
                case "bos":
                    {
                        outstation.Transaction(db =>
                        {
                            binaryOutputStatusValue = !binaryOutputStatusValue;
                            db.UpdateBinaryOutputStatus(new BinaryOutputStatus(7, binaryOutputStatusValue, new Flags(Flag.Online), Now()), new UpdateOptions());
                        });
                        break;
                    }
                case "co":
                    {
                        outstation.Transaction(db =>
                        {
                            db.UpdateCounter(new Counter(7, ++counterValue, new Flags(Flag.Online), Now()), new UpdateOptions());
                        });
                        break;
                    }
                case "fco":
                    {
                        outstation.Transaction(db =>
                        {
                            db.UpdateFrozenCounter(new FrozenCounter(7, ++frozenCounterValue, new Flags(Flag.Online), Now()), new UpdateOptions());
                        });
                        break;
                    }
                case "ai":
                    {
                        outstation.Transaction(db =>
                        {
                            db.UpdateAnalog(new AnalogInput(7, ++analogValue, new Flags(Flag.Online), Now()), new UpdateOptions());
                        });
                        break;
                    }
                case "aos":
                    {
                        outstation.Transaction(db =>
                        {
                            db.UpdateAnalogOutputStatus(new AnalogOutputStatus(7, ++analogOutputStatusValue, new Flags(Flag.Online), Now()), new UpdateOptions());
                        });
                        break;
                    }
                case "os":
                    {
                        outstation.Transaction(db =>
                        {
                            db.UpdateOctetString(7, System.Text.Encoding.ASCII.GetBytes("Hello"), new UpdateOptions());
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
