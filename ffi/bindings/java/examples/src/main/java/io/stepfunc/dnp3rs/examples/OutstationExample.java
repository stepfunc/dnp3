package io.stepfunc.dnp3rs.examples;

import io.stepfunc.dnp3rs.Runtime;
import io.stepfunc.dnp3rs.*;
import org.joou.UByte;
import org.joou.UShort;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

import static org.joou.Unsigned.*;

class TestLogger implements Logger {

    @Override
    public void onMessage(LogLevel level, String message) {
        System.out.print(message);
    }
}

class TestApplication implements OutstationApplication {

    @Override
    public UShort getProcessingDelayMs() {
        return ushort(0);
    }

    @Override
    public RestartDelay coldRestart() {
        return RestartDelay.validSeconds(ushort(60));
    }

    @Override
    public RestartDelay warmRestart() {
        return RestartDelay.notSupported();
    }
}

class TestOutstationInformation implements OutstationInformation {

    @Override
    public void processRequestFromIdle(RequestHeader header) {

    }

    @Override
    public void broadcastReceived(FunctionCode functionCode, BroadcastAction action) {

    }

    @Override
    public void enterSolicitedConfirmWait(UByte ecsn) {

    }

    @Override
    public void solicitedConfirmTimeout(UByte ecsn) {

    }

    @Override
    public void solicitedConfirmReceived(UByte ecsn) {

    }

    @Override
    public void solicitedConfirmWaitNewRequest(RequestHeader header) {

    }

    @Override
    public void wrongSolicitedConfirmSeq(UByte ecsn, UByte seq) {

    }

    @Override
    public void unexpectedConfirm(boolean unsolicited, UByte seq) {

    }

    @Override
    public void enterUnsolicitedConfirmWait(UByte ecsn) {

    }

    @Override
    public void unsolicitedConfirmTimeout(UByte ecsn, boolean retry) {

    }

    @Override
    public void unsolicitedConfirmed(UByte ecsn) {

    }

    @Override
    public void clearRestartIin() {

    }
}

class TestControlHandler implements ControlHandler {

    @Override
    public void beginFragment() {

    }

    @Override
    public void endFragment() {

    }

    @Override
    public CommandStatus selectG12v1(G12v1 control, UShort index, Database database) {
        return CommandStatus.NOT_SUPPORTED;
    }

    @Override
    public CommandStatus operateG12v1(G12v1 control, UShort index, OperateType opType, Database database) {
        return CommandStatus.NOT_SUPPORTED;
    }

    @Override
    public CommandStatus selectG41v1(int control, UShort index, Database database) {
        return CommandStatus.NOT_SUPPORTED;
    }

    @Override
    public CommandStatus operateG41v1(int control, UShort index, OperateType opType, Database database) {
        return CommandStatus.NOT_SUPPORTED;
    }

    @Override
    public CommandStatus selectG41v2(short value, UShort index, Database database) {
        return CommandStatus.NOT_SUPPORTED;
    }

    @Override
    public CommandStatus operateG41v2(short value, UShort index, OperateType opType, Database database) {
        return CommandStatus.NOT_SUPPORTED;
    }

    @Override
    public CommandStatus selectG41v3(float value, UShort index, Database database) {
        return CommandStatus.NOT_SUPPORTED;
    }

    @Override
    public CommandStatus operateG41v3(float value, UShort index, OperateType opType, Database database) {
        return CommandStatus.NOT_SUPPORTED;
    }

    @Override
    public CommandStatus selectG41v4(double value, UShort index, Database database) {
        return CommandStatus.NOT_SUPPORTED;
    }

    @Override
    public CommandStatus operateG41v4(double value, UShort index, OperateType opType, Database database) {
        return CommandStatus.NOT_SUPPORTED;
    }
}

public class OutstationExample {

    static LoggingConfiguration getLoggingConfig() {
        LoggingConfiguration config = new LoggingConfiguration();
        config.level = LogLevel.INFO;
        config.printLevel = true;
        config.printModuleInfo = false;
        config.timeFormat = TimeFormat.SYSTEM;
        config.outputFormat = LogOutputFormat.TEXT;
        return config;
    }

    public static void main(String[] args) {
        // Setup logging
        Logging.configure(getLoggingConfig(), new TestLogger());

        // Create the tokio runtime
        RuntimeConfig runtimeConfig = new RuntimeConfig();
        runtimeConfig.numCoreThreads = ushort(4);
        try(Runtime runtime = new Runtime(runtimeConfig)) {
            TcpServer server = new TcpServer(runtime, "127.0.0.1:20000");

            OutstationConfig config = OutstationConfig.defaultConfig(ushort(1024), ushort(1));
            config.logLevel = DecodeLogLevel.OBJECT_VALUES;
            DatabaseConfig database = DatabaseConfig.defaultConfig();
            database.events = EventBufferConfig.allTypes(ushort(10));
            TestApplication application = new TestApplication();
            TestOutstationInformation information = new TestOutstationInformation();
            TestControlHandler controlHandler = new TestControlHandler();
            AddressFilter addressFilter = AddressFilter.any();
            Outstation outstation = server.addOutstation(config, database, application, information, controlHandler, addressFilter);

            // Setup initial points
            outstation.transaction((db) -> {
                for(int i = 0; i < 10; i++) {
                    BinaryConfig binaryConfig = new BinaryConfig();
                    binaryConfig.staticVariation = StaticBinaryVariation.GROUP1_VAR2;
                    binaryConfig.eventVariation = EventBinaryVariation.GROUP2_VAR2;
                    db.addBinary(ushort(i), EventClass.CLASS1, binaryConfig);

                    DoubleBitBinaryConfig doubleBitBinaryConfig = new DoubleBitBinaryConfig();
                    doubleBitBinaryConfig.staticVariation = StaticDoubleBitBinaryVariation.GROUP3_VAR2;
                    doubleBitBinaryConfig.eventVariation = EventDoubleBitBinaryVariation.GROUP4_VAR2;
                    db.addDoubleBitBinary(ushort(i), EventClass.CLASS1, doubleBitBinaryConfig);

                    BinaryOutputStatusConfig binaryOutputStatusConfig = new BinaryOutputStatusConfig();
                    binaryOutputStatusConfig.staticVariation = StaticBinaryOutputStatusVariation.GROUP10_VAR2;
                    binaryOutputStatusConfig.eventVariation = EventBinaryOutputStatusVariation.GROUP11_VAR2;
                    db.addBinaryOutputStatus(ushort(i), EventClass.CLASS1, binaryOutputStatusConfig);

                    CounterConfig counterConfig = new CounterConfig();
                    counterConfig.staticVariation = StaticCounterVariation.GROUP20_VAR1;
                    counterConfig.eventVariation = EventCounterVariation.GROUP22_VAR1;
                    counterConfig.deadband = uint(0);
                    db.addCounter(ushort(i), EventClass.CLASS1, counterConfig);

                    FrozenCounterConfig frozenCounterConfig = new FrozenCounterConfig();
                    frozenCounterConfig.staticVariation = StaticFrozenCounterVariation.GROUP21_VAR5;
                    frozenCounterConfig.eventVariation = EventFrozenCounterVariation.GROUP23_VAR5;
                    frozenCounterConfig.deadband = uint(0);
                    db.addFrozenCounter(ushort(i), EventClass.CLASS1, frozenCounterConfig);

                    AnalogConfig analogConfig = new AnalogConfig();
                    analogConfig.staticVariation = StaticAnalogVariation.GROUP30_VAR6;
                    analogConfig.eventVariation = EventAnalogVariation.GROUP32_VAR8;
                    analogConfig.deadband = 0.0;
                    db.addAnalog(ushort(i), EventClass.CLASS1, analogConfig);

                    AnalogOutputStatusConfig analogOutputStatusConfig = new AnalogOutputStatusConfig();
                    analogOutputStatusConfig.staticVariation = StaticAnalogOutputStatusVariation.GROUP40_VAR4;
                    analogOutputStatusConfig.eventVariation = EventAnalogOutputStatusVariation.GROUP42_VAR8;
                    analogOutputStatusConfig.deadband = 0.0;
                    db.addAnalogOutputStatus(ushort(i), EventClass.CLASS1, analogOutputStatusConfig);

                    db.addOctetString(ushort(i), EventClass.CLASS1);

                    Flags flags = new Flags();
                    flags.value = ubyte(0x00);
                    flags = flags.set(Flag.RESTART, true);

                    Binary binaryPoint = new Binary();
                    binaryPoint.index = ushort(i);
                    binaryPoint.value = false;
                    binaryPoint.flags = flags;
                    binaryPoint.time = Timestamp.invalidTimestamp();
                    db.updateBinary(binaryPoint, UpdateOptions.defaultOptions());

                    DoubleBitBinary doubleBitBinaryPoint = new DoubleBitBinary();
                    doubleBitBinaryPoint.index = ushort(i);
                    doubleBitBinaryPoint.value = DoubleBit.INDETERMINATE;
                    doubleBitBinaryPoint.flags = flags;
                    doubleBitBinaryPoint.time = Timestamp.invalidTimestamp();
                    db.updateDoubleBitBinary(doubleBitBinaryPoint, UpdateOptions.defaultOptions());

                    BinaryOutputStatus binaryOutputStatusPoint = new BinaryOutputStatus();
                    binaryOutputStatusPoint.index = ushort(i);
                    binaryOutputStatusPoint.value = false;
                    binaryOutputStatusPoint.flags = flags;
                    binaryOutputStatusPoint.time = Timestamp.invalidTimestamp();
                    db.updateBinaryOutputStatus(binaryOutputStatusPoint, UpdateOptions.defaultOptions());

                    Counter counterPoint = new Counter();
                    counterPoint.index = ushort(i);
                    counterPoint.value = uint(0);
                    counterPoint.flags = flags;
                    counterPoint.time = Timestamp.invalidTimestamp();
                    db.updateCounter(counterPoint, UpdateOptions.defaultOptions());

                    FrozenCounter frozenCounterPoint = new FrozenCounter();
                    frozenCounterPoint.index = ushort(i);
                    frozenCounterPoint.value = uint(0);
                    frozenCounterPoint.flags = flags;
                    frozenCounterPoint.time = Timestamp.invalidTimestamp();
                    db.updateFrozenCounter(frozenCounterPoint, UpdateOptions.defaultOptions());

                    Analog analogPoint = new Analog();
                    analogPoint.index = ushort(i);
                    analogPoint.value = 0.0;
                    analogPoint.flags = flags;
                    analogPoint.time = Timestamp.invalidTimestamp();
                    db.updateAnalog(analogPoint, UpdateOptions.defaultOptions());

                    AnalogOutputStatus analogOutputStatusPoint = new AnalogOutputStatus();
                    analogOutputStatusPoint.index = ushort(i);
                    analogOutputStatusPoint.value = 0.0;
                    analogOutputStatusPoint.flags = flags;
                    analogOutputStatusPoint.time = Timestamp.invalidTimestamp();
                    db.updateAnalogOutputStatus(analogOutputStatusPoint, UpdateOptions.defaultOptions());
                }
            });

            // Start the outstation
            server.bind();

            boolean binaryValue = false;
            DoubleBit doubleBitBinaryValue = DoubleBit.DETERMINED_OFF;
            boolean binaryOutputStatusValue = false;
            long counterValue = 0;
            long frozenCounterValue = 0;
            double analogValue = 0.0;
            double analogOutputStatusValue = 0.0;
            Flags tmpFlags = new Flags();
            tmpFlags.value = ubyte(0x00);
            final Flags onlineFlags = tmpFlags.set(Flag.ONLINE, true);

            // Handle user input
            try {
                BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
                while (true) {
                    String line = reader.readLine();
                    switch(line) {
                        case "x":
                            return;
                        case "bi":
                        {
                            binaryValue = !binaryValue;
                            final boolean pointValue = binaryValue;
                            outstation.transaction((db) -> {
                                Binary value = new Binary();
                                value.index = ushort(7);
                                value.value = pointValue;
                                value.flags = onlineFlags;
                                value.time = Timestamp.synchronizedTimestamp(ulong(0));
                                db.updateBinary(value, UpdateOptions.defaultOptions());
                            });
                            break;
                        }
                        case "dbbi":
                        {
                            doubleBitBinaryValue = doubleBitBinaryValue == DoubleBit.DETERMINED_OFF ? DoubleBit.DETERMINED_ON : DoubleBit.DETERMINED_OFF;
                            final DoubleBit pointValue = doubleBitBinaryValue;
                            outstation.transaction((db) -> {
                                DoubleBitBinary value = new DoubleBitBinary();
                                value.index = ushort(7);
                                value.value = pointValue;
                                value.flags = onlineFlags;
                                value.time = Timestamp.synchronizedTimestamp(ulong(0));
                                db.updateDoubleBitBinary(value, UpdateOptions.defaultOptions());
                            });
                            break;
                        }
                        case "bos":
                        {
                            binaryOutputStatusValue = !binaryOutputStatusValue;
                            final boolean pointValue = binaryOutputStatusValue;
                            outstation.transaction((db) -> {
                                BinaryOutputStatus value = new BinaryOutputStatus();
                                value.index = ushort(7);
                                value.value = pointValue;
                                value.flags = onlineFlags;
                                value.time = Timestamp.synchronizedTimestamp(ulong(0));
                                db.updateBinaryOutputStatus(value, UpdateOptions.defaultOptions());
                            });
                            break;
                        }
                        case "co":
                        {
                            counterValue = ++counterValue;
                            final long pointValue = counterValue;
                            outstation.transaction((db) -> {
                                Counter value = new Counter();
                                value.index = ushort(7);
                                value.value = uint(pointValue);
                                value.flags = onlineFlags;
                                value.time = Timestamp.synchronizedTimestamp(ulong(0));
                                db.updateCounter(value, UpdateOptions.defaultOptions());
                            });
                            break;
                        }
                        case "fco":
                        {
                            frozenCounterValue = ++frozenCounterValue;
                            final long pointValue = frozenCounterValue;
                            outstation.transaction((db) -> {
                                FrozenCounter value = new FrozenCounter();
                                value.index = ushort(7);
                                value.value = uint(pointValue);
                                value.flags = onlineFlags;
                                value.time = Timestamp.synchronizedTimestamp(ulong(0));
                                db.updateFrozenCounter(value, UpdateOptions.defaultOptions());
                            });
                            break;
                        }
                        case "ai":
                        {
                            analogValue = ++analogValue;
                            final double pointValue = analogValue;
                            outstation.transaction((db) -> {
                                Analog value = new Analog();
                                value.index = ushort(7);
                                value.value = pointValue;
                                value.flags = onlineFlags;
                                value.time = Timestamp.synchronizedTimestamp(ulong(0));
                                db.updateAnalog(value, UpdateOptions.defaultOptions());
                            });
                            break;
                        }
                        case "aos":
                        {
                            analogOutputStatusValue = ++analogOutputStatusValue;
                            final double pointValue = analogOutputStatusValue;
                            outstation.transaction((db) -> {
                                AnalogOutputStatus value = new AnalogOutputStatus();
                                value.index = ushort(7);
                                value.value = pointValue;
                                value.flags = onlineFlags;
                                value.time = Timestamp.synchronizedTimestamp(ulong(0));
                                db.updateAnalogOutputStatus(value, UpdateOptions.defaultOptions());
                            });
                            break;
                        }
                        case "os":
                        {
                            outstation.transaction((db) -> {
                                // Some Friday poetry:
                                // Arrays.asList requires an array of Object,
                                // Arrays.stream does not overload for byte[], how abject.
                                // Java is poop
                                // So excuse me for that for loop
                                List<UByte> octetString = new ArrayList<>();
                                for(byte octet : "Hello".getBytes(StandardCharsets.US_ASCII)) {
                                    octetString.add(ubyte(octet));
                                }

                                db.updateOctetString(ushort(7), octetString, UpdateOptions.defaultOptions());
                            });
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
