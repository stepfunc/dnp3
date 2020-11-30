package io.stepfunc.dnp3rs_tests;

import io.stepfunc.dnp3rs.*;
import io.stepfunc.dnp3rs.Runtime;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.time.Duration;
import java.util.List;

import static org.joou.Unsigned.*;

class TestLogger implements Logger {

    @Override
    public void onMessage(LogLevel level, String message) {
        System.out.println(level + ": " + message);
    }
}

class TestListener implements ClientStateListener {

    @Override
    public void onChange(ClientState state) {
        System.out.println(state);
    }
}

class TestReadHandler implements ReadHandler {

    @Override
    public void beginFragment(ResponseHeader header) {
        System.out.println("Beginning fragment (broadcast: " + header.iin.iin1.isSet(Iin1Flag.BROADCAST) + ")");
    }

    @Override
    public void endFragment(ResponseHeader header) {
        System.out.println("End fragment");
    }

    @Override
    public void handleBinary(HeaderInfo info, List<Binary> it) {
        System.out.println("Binaries:");
        System.out.println("Qualifier: " + info.qualifier);
        System.out.println("Variation: " + info.variation);

        it.forEach(val -> {
            System.out.println("BI " + val.index + ": Value=" + val.value + " Flags=" + val.flags.value + " Time=" + val.time.value + " (" + val.time.quality + ")");
        });
    }

    @Override
    public void handleDoubleBitBinary(HeaderInfo info, List<DoubleBitBinary> it) {
        System.out.println("Double Bit Binaries:");
        System.out.println("Qualifier: " + info.qualifier);
        System.out.println("Variation: " + info.variation);

        it.forEach(val -> {
            System.out.println("DBBI " + val.index + ": Value=" + val.value + " Flags=" + val.flags.value + " Time=" + val.time.value + " (" + val.time.quality + ")");
        });
    }

    @Override
    public void handleBinaryOutputStatus(HeaderInfo info, List<BinaryOutputStatus> it) {
        System.out.println("Binary Output Statuses:");
        System.out.println("Qualifier: " + info.qualifier);
        System.out.println("Variation: " + info.variation);

        it.forEach(val -> {
            System.out.println("BOS " + val.index + ": Value=" + val.value + " Flags=" + val.flags.value + " Time=" + val.time.value + " (" + val.time.quality + ")");
        });
    }

    @Override
    public void handleCounter(HeaderInfo info, List<Counter> it) {
        System.out.println("Counters:");
        System.out.println("Qualifier: " + info.qualifier);
        System.out.println("Variation: " + info.variation);

        it.forEach(val -> {
            System.out.println("Counter " + val.index + ": Value=" + val.value + " Flags=" + val.flags.value + " Time=" + val.time.value + " (" + val.time.quality + ")");
        });
    }

    @Override
    public void handleFrozenCounter(HeaderInfo info, List<FrozenCounter> it) {
        System.out.println("Frozen Counters:");
        System.out.println("Qualifier: " + info.qualifier);
        System.out.println("Variation: " + info.variation);

        it.forEach(val -> {
            System.out.println("Frozen Counter " + val.index + ": Value=" + val.value + " Flags=" + val.flags.value + " Time=" + val.time.value + " (" + val.time.quality + ")");
        });
    }

    @Override
    public void handleAnalog(HeaderInfo info, List<Analog> it) {
        System.out.println("Analogs:");
        System.out.println("Qualifier: " + info.qualifier);
        System.out.println("Variation: " + info.variation);

        it.forEach(val -> {
            System.out.println("AI " + val.index + ": Value=" + val.value + " Flags=" + val.flags.value + " Time=" + val.time.value + " (" + val.time.quality + ")");
        });
    }

    @Override
    public void handleAnalogOutputStatus(HeaderInfo info, List<AnalogOutputStatus> it) {
        System.out.println("Analog Output Statuses:");
        System.out.println("Qualifier: " + info.qualifier);
        System.out.println("Variation: " + info.variation);

        it.forEach(val -> {
            System.out.println("AOS " + val.index + ": Value=" + val.value + " Flags=" + val.flags.value + " Time=" + val.time.value + " (" + val.time.quality + ")");
        });
    }

    @Override
    public void handleOctetString(HeaderInfo info, List<OctetString> it) {
        System.out.println("Octet String:");
        System.out.println("Qualifier: " + info.qualifier);
        System.out.println("Variation: " + info.variation);

        it.forEach(val -> {
            System.out.print("Octet String " + val.index + ": Value=");
            val.value.forEach(b -> System.out.print(String.format("%02X", b.value.byteValue()) + " "));
            System.out.println();
        });
    }
}

class TestTimeProvider implements TimeProvider
{
    @Override
    public TimeProviderTimestamp getTime() {
        return TimeProviderTimestamp.valid(ulong(System.currentTimeMillis()));
    }
}

public class Main {
    public static void main(String[] args) {
        // Setup logging
        Logging.setLogLevel(LogLevel.INFO);
        Logging.setHandler(new TestLogger());

        // Create the tokio runtime
        RuntimeConfig runtimeConfig = new RuntimeConfig();
        runtimeConfig.numCoreThreads = ushort(4);
        try(Runtime runtime = new Runtime(runtimeConfig)) {
            // Create the master
            RetryStrategy retryStrategy = new RetryStrategy();
            retryStrategy.minDelay = Duration.ofMillis(100);
            retryStrategy.maxDelay = Duration.ofSeconds(5);
            Master master = runtime.addMasterTcp(
                    ushort(1),
                    DecodeLogLevel.OBJECT_VALUES,
                    retryStrategy,
                    Duration.ofSeconds(5),
                    "127.0.0.1:20000",
                    new TestListener()
            );

            // Create the association
            TestReadHandler readHandler = new TestReadHandler();
            AssociationConfiguration associationConfiguration = new AssociationConfiguration();
            associationConfiguration.disableUnsolClasses = new EventClasses();
            associationConfiguration.disableUnsolClasses.class1 = true;
            associationConfiguration.disableUnsolClasses.class2 = true;
            associationConfiguration.disableUnsolClasses.class3 = true;
            associationConfiguration.enableUnsolClasses = new EventClasses();
            associationConfiguration.enableUnsolClasses.class1 = true;
            associationConfiguration.enableUnsolClasses.class2 = true;
            associationConfiguration.enableUnsolClasses.class3 = true;
            associationConfiguration.autoTimeSync = AutoTimeSync.LAN;
            associationConfiguration.autoTasksRetryStrategy = retryStrategy;
            AssociationHandlers associationHandlers = new AssociationHandlers();
            associationHandlers.integrityHandler = readHandler;
            associationHandlers.unsolicitedHandler = readHandler;
            associationHandlers.defaultPollHandler = readHandler;
            Association association = master.addAssociation(
                    ushort(1024),
                    associationConfiguration,
                    associationHandlers,
                    new TestTimeProvider()
            );

            // Create a periodic poll
            Request pollRequest = Request.classRequest(false, true, true, true);
            Poll poll = association.addPoll(pollRequest, Duration.ofSeconds(5));

            // Handle user input
            try {
                BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
                while (true) {
                    String line = reader.readLine();
                    switch(line) {
                        case "x":
                            return;
                        case "dln":
                            master.setDecodeLogLevel(DecodeLogLevel.NOTHING);
                            break;
                        case "dlv":
                            master.setDecodeLogLevel(DecodeLogLevel.OBJECT_VALUES);
                            break;
                        case "rao":
                        {
                            Request request = new Request();
                            request.addAllObjectsHeader(Variation.GROUP40_VAR0);
                            ReadResult result = association.read(request).toCompletableFuture().get();
                            System.out.println("Result: " + result);
                            break;
                        }
                        case "rmo":
                        {
                            Request request = new Request();
                            request.addAllObjectsHeader(Variation.GROUP10_VAR0);
                            request.addAllObjectsHeader(Variation.GROUP40_VAR0);
                            ReadResult result = association.read(request).toCompletableFuture().get();
                            System.out.println("Result: " + result);
                            break;
                        }
                        case "cmd":
                        {
                            Command command = new Command();
                            G12v1 g12v1 = new G12v1();
                            ControlCode code = new ControlCode();
                            code.tcc = TripCloseCode.NUL;
                            code.clear = false;
                            code.queue = false;
                            code.opType = OpType.LATCH_ON;
                            g12v1.code = code;
                            g12v1.count = ubyte(1);
                            g12v1.onTime = uint(1000);
                            g12v1.offTime = uint(1000);
                            command.addU16g12v1(ushort(3), g12v1);
                            CommandResult result = association.operate(CommandMode.SELECT_BEFORE_OPERATE, command).toCompletableFuture().get();
                            System.out.println("Result: " + result);
                            break;
                        }
                        case "evt":
                            poll.demand();
                            break;
                        case "lts":
                        {
                            TimeSyncResult result = association.performTimeSync(TimeSyncMode.LAN).toCompletableFuture().get();
                            System.out.println("Result: " + result);
                            break;
                        }
                        case "nts":
                        {
                            TimeSyncResult result = association.performTimeSync(TimeSyncMode.NON_LAN).toCompletableFuture().get();
                            System.out.println("Result: " + result);
                            break;
                        }
                        default:
                            System.out.println("Unknown command");
                            break;
                    }
                }
            } catch(Exception ex) {
                System.out.println(ex.getMessage());
            }
        }
    }
}
