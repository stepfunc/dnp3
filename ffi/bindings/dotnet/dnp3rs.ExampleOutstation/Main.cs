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

        public CommandStatus SelectG12v1(G12v1 control, ushort index) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG12v1(G12v1 control, ushort index, OperateType opType) { return CommandStatus.NotSupported; }

        public CommandStatus SelectG41v1(int control, ushort index) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG41v1(int control, ushort index, OperateType opType) { return CommandStatus.NotSupported; }

        public CommandStatus SelectG41v2(short value, ushort index) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG41v2(short value, ushort index, OperateType opType) { return CommandStatus.NotSupported; }

        public CommandStatus SelectG41v3(float value, ushort index) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG41v3(float value, ushort index, OperateType opType) { return CommandStatus.NotSupported; }

        public CommandStatus SelectG41v4(double value, ushort index) { return CommandStatus.NotSupported; }

        public CommandStatus OperateG41v4(double value, ushort index, OperateType opType) { return CommandStatus.NotSupported; }
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
                var application = new TestOutstationApplication();
                var information = new TestOutstationInformation();
                var controlHandler = new TestControlHandler();
                var addressFilter = AddressFilter.Any();
                var outstation = server.AddOutstation(config, database, application, information, controlHandler, addressFilter);

                server.Bind();

                while (true)
                {
                    switch (await GetInputAsync())
                    {
                        case "x":
                            return;
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
