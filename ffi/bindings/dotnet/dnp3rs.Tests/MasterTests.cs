using System;
using Xunit;
using dnp3rs;

namespace dnp3rs.Tests
{
    public class MasterTests
    {
        class TestListener : ClientStateListener
        {
            public void OnChange(ClientState state)
            {
                Console.WriteLine(state);
            }
        }

        [Fact]
        public void DurationZeroTest()
        {
            var config = new RuntimeConfig();
            config.NumCoreThreads = 2;

            using(var runtime = new Runtime(config))
            {
                var strategy = new ReconnectStrategy();
                strategy.MinDelay = TimeSpan.FromMilliseconds(100);
                strategy.MaxDelay = TimeSpan.FromSeconds(5);
                var master = runtime.AddMasterTcp(1024, DecodeLogLevel.ObjectValues, strategy, TimeSpan.FromSeconds(5), "127.0.0.1:8000", new TestListener());
            }
        }
    }
}
