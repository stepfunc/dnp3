// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package io.stepfunc.dnp3.tests;

import static org.joou.Unsigned.*;
import static org.junit.jupiter.api.Assertions.assertEquals;

import io.stepfunc.dnp3.*;
import io.stepfunc.dnp3.Runtime;
import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;
import org.joou.UByte;
import org.joou.UShort;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Timeout;

// Smoke test of the Java bindings: an outstation and a master in one process, talking over TCP
// loopback. It checks that the library loads and that calls, callbacks and async completions
// cross the boundary: a class 0 read returns the outstation's values, and a direct operate
// reaches the outstation's control handler.
class SmokeTest {
  static final String ENDPOINT = "127.0.0.1:20104";
  static final UShort OUTSTATION_ADDRESS = ushort(1024);
  static final UShort MASTER_ADDRESS = ushort(1);
  static final long WAIT_TIMEOUT_SECONDS = 10;

  static class Control {
    final UShort index;
    final Group12Var1 value;
    final OperateType operateType;

    Control(UShort index, Group12Var1 value, OperateType operateType) {
      this.index = index;
      this.value = value;
      this.operateType = operateType;
    }
  }

  static class TestOutstationApplication implements OutstationApplication {
    @Override
    public UShort getProcessingDelayMs() {
      return ushort(0);
    }

    @Override
    public WriteTimeResult writeAbsoluteTime(org.joou.ULong time) {
      return WriteTimeResult.NOT_SUPPORTED;
    }

    @Override
    public ApplicationIin getApplicationIin() {
      return new ApplicationIin();
    }

    @Override
    public RestartDelay coldRestart() {
      return RestartDelay.notSupported();
    }

    @Override
    public RestartDelay warmRestart() {
      return RestartDelay.notSupported();
    }
  }

  static class TestOutstationInformation implements OutstationInformation {
    @Override
    public void processRequestFromIdle(RequestHeader header) {}

    @Override
    public void broadcastReceived(FunctionCode functionCode, BroadcastAction action) {}

    @Override
    public void enterSolicitedConfirmWait(UByte ecsn) {}

    @Override
    public void solicitedConfirmTimeout(UByte ecsn) {}

    @Override
    public void solicitedConfirmReceived(UByte ecsn) {}

    @Override
    public void solicitedConfirmWaitNewRequest() {}

    @Override
    public void wrongSolicitedConfirmSeq(UByte ecsn, UByte seq) {}

    @Override
    public void unexpectedConfirm(boolean unsolicited, UByte seq) {}

    @Override
    public void enterUnsolicitedConfirmWait(UByte ecsn) {}

    @Override
    public void unsolicitedConfirmTimeout(UByte ecsn, boolean retry) {}

    @Override
    public void unsolicitedConfirmed(UByte ecsn) {}

    @Override
    public void clearRestartIin() {}
  }

  static class TestControlHandler implements ControlHandler {
    final CompletableFuture<Control> received = new CompletableFuture<>();

    @Override
    public void beginFragment() {}

    @Override
    public void endFragment(DatabaseHandle database) {}

    @Override
    public CommandStatus selectG12v1(Group12Var1 value, UShort index, DatabaseHandle database) {
      return CommandStatus.NOT_SUPPORTED;
    }

    @Override
    public CommandStatus operateG12v1(
        Group12Var1 value, UShort index, OperateType opType, DatabaseHandle database) {
      received.complete(new Control(index, value, opType));
      return CommandStatus.SUCCESS;
    }

    @Override
    public CommandStatus selectG41v1(int value, UShort index, DatabaseHandle database) {
      return CommandStatus.NOT_SUPPORTED;
    }

    @Override
    public CommandStatus operateG41v1(
        int value, UShort index, OperateType opType, DatabaseHandle database) {
      return CommandStatus.NOT_SUPPORTED;
    }

    @Override
    public CommandStatus selectG41v2(short value, UShort index, DatabaseHandle database) {
      return CommandStatus.NOT_SUPPORTED;
    }

    @Override
    public CommandStatus operateG41v2(
        short value, UShort index, OperateType opType, DatabaseHandle database) {
      return CommandStatus.NOT_SUPPORTED;
    }

    @Override
    public CommandStatus selectG41v3(float value, UShort index, DatabaseHandle database) {
      return CommandStatus.NOT_SUPPORTED;
    }

    @Override
    public CommandStatus operateG41v3(
        float value, UShort index, OperateType opType, DatabaseHandle database) {
      return CommandStatus.NOT_SUPPORTED;
    }

    @Override
    public CommandStatus selectG41v4(double value, UShort index, DatabaseHandle database) {
      return CommandStatus.NOT_SUPPORTED;
    }

    @Override
    public CommandStatus operateG41v4(
        double value, UShort index, OperateType opType, DatabaseHandle database) {
      return CommandStatus.NOT_SUPPORTED;
    }
  }

  // records binary input 0 and analog input 0; the read completes after the handler runs
  static class TestReadHandler implements ReadHandler {
    Boolean binaryValue;
    Double analogValue;

    @Override
    public synchronized void handleBinaryInput(HeaderInfo info, List<BinaryInput> values) {
      for (BinaryInput value : values) {
        if (value.index.intValue() == 0) {
          binaryValue = value.value;
        }
      }
    }

    @Override
    public synchronized void handleAnalogInput(HeaderInfo info, List<AnalogInput> values) {
      for (AnalogInput value : values) {
        if (value.index.intValue() == 0) {
          analogValue = value.value;
        }
      }
    }

    synchronized Boolean getBinaryValue() {
      return binaryValue;
    }

    synchronized Double getAnalogValue() {
      return analogValue;
    }
  }

  static class TestAssociationInformation implements AssociationInformation {
    @Override
    public void taskStart(TaskType taskType, FunctionCode functionCode, UByte seq) {}

    @Override
    public void taskSuccess(TaskType taskType, FunctionCode functionCode, UByte seq) {}

    @Override
    public void taskFail(TaskType taskType, TaskError error) {}

    @Override
    public void unsolicitedResponse(boolean isDuplicate, UByte seq) {}
  }

  @Test
  @Timeout(60)
  void masterReadsAndOperatesOutstation() throws Exception {
    Runtime runtime = new Runtime(new RuntimeConfig().withNumCoreThreads(ushort(2)));
    try {
      OutstationServer server =
          OutstationServer.createTcpServer(runtime, LinkErrorMode.CLOSE, ENDPOINT);
      TestControlHandler controlHandler = new TestControlHandler();
      Outstation outstation =
          server.addOutstation(
              new OutstationConfig(
                  OUTSTATION_ADDRESS, MASTER_ADDRESS, EventBufferConfig.noEvents()),
              new TestOutstationApplication(),
              new TestOutstationInformation(),
              controlHandler,
              state -> {},
              AddressFilter.any());
      outstation.transaction(
          db -> {
            Flags online = new Flags(Flag.ONLINE);
            db.addBinaryInput(ushort(0), EventClass.NONE, new BinaryInputConfig());
            db.addAnalogInput(ushort(0), EventClass.NONE, new AnalogInputConfig());
            db.updateBinaryInput(
                new BinaryInput(ushort(0), true, online, Timestamp.invalidTimestamp()),
                UpdateOptions.detectEvent());
            db.updateAnalogInput(
                new AnalogInput(ushort(0), 42.0, online, Timestamp.invalidTimestamp()),
                UpdateOptions.detectEvent());
          });
      server.bind();

      CompletableFuture<Void> connected = new CompletableFuture<>();
      MasterChannel channel =
          MasterChannel.createTcpChannel(
              runtime,
              LinkErrorMode.CLOSE,
              new MasterChannelConfig(MASTER_ADDRESS),
              new EndpointList(ENDPOINT),
              new ConnectStrategy(),
              state -> {
                if (state == ClientState.CONNECTED) {
                  connected.complete(null);
                }
              });
      // no startup integrity poll or unsolicited setup, so only the test's own requests are made
      TestReadHandler readHandler = new TestReadHandler();
      AssociationId association =
          channel.addAssociation(
              OUTSTATION_ADDRESS,
              new AssociationConfig(
                  EventClasses.none(), EventClasses.none(), Classes.none(), EventClasses.none()),
              readHandler,
              UtcTimestamp::invalid,
              new TestAssociationInformation());
      channel.enable();
      connected.get(WAIT_TIMEOUT_SECONDS, TimeUnit.SECONDS);

      // a class 0 read returns the outstation's values
      channel
          .read(association, Request.classRequest(true, false, false, false))
          .toCompletableFuture()
          .get(WAIT_TIMEOUT_SECONDS, TimeUnit.SECONDS);
      assertEquals(Boolean.TRUE, readHandler.getBinaryValue());
      assertEquals(Double.valueOf(42.0), readHandler.getAnalogValue());

      // a direct operate reaches the outstation's control handler
      CommandSet commands = new CommandSet();
      commands.addG12V1U16(
          ushort(0),
          new Group12Var1(
              new ControlCode(TripCloseCode.NUL, false, OpType.LATCH_ON),
              ubyte(1),
              uint(1000),
              uint(2000)));
      channel
          .operate(association, CommandMode.DIRECT_OPERATE, commands)
          .toCompletableFuture()
          .get(WAIT_TIMEOUT_SECONDS, TimeUnit.SECONDS);
      Control control = controlHandler.received.get(WAIT_TIMEOUT_SECONDS, TimeUnit.SECONDS);
      assertEquals(ushort(0), control.index);
      assertEquals(OpType.LATCH_ON, control.value.code.opType);
      assertEquals(OperateType.DIRECT_OPERATE, control.operateType);
      assertEquals(ubyte(1), control.value.count);
      assertEquals(uint(1000), control.value.onTime);
      assertEquals(uint(2000), control.value.offTime);

      channel.shutdown();
      server.shutdown();
    } finally {
      runtime.shutdown();
    }
  }
}
