// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Microsoft.VisualStudio.TestTools.UnitTesting;
using dnp3;

namespace dnp3_tests
{
    // Smoke test of the .NET bindings: an outstation and a master in one process, talking over TCP
    // loopback. It checks that the library loads and that calls, callbacks and async completions
    // cross the boundary: a class 0 read returns the outstation's values, and a direct operate
    // reaches the outstation's control handler.
    [TestClass]
    public class SmokeTest
    {
        const string Endpoint = "127.0.0.1:20103";
        const ushort OutstationAddress = 1024;
        const ushort MasterAddress = 1;
        static readonly TimeSpan WaitTimeout = TimeSpan.FromSeconds(10);

        class Control
        {
            public ushort Index;
            public OpType OpType;
            public OperateType OperateType;
            public byte Count;
            public uint OnTime;
            public uint OffTime;
        }

        class OutstationApplication : IOutstationApplication
        {
            public ushort GetProcessingDelayMs() => 0;
            public WriteTimeResult WriteAbsoluteTime(ulong time) => WriteTimeResult.NotSupported;
            public ApplicationIin GetApplicationIin() => new ApplicationIin();
            public RestartDelay ColdRestart() => RestartDelay.NotSupported();
            public RestartDelay WarmRestart() => RestartDelay.NotSupported();
            public FreezeResult FreezeCountersAll(FreezeType freezeType, DatabaseHandle databaseHandle) => FreezeResult.NotSupported;
            public FreezeResult FreezeCountersAllAtTime(DatabaseHandle databaseHandle, ulong time, uint interval) => FreezeResult.NotSupported;
            public FreezeResult FreezeCountersRange(ushort start, ushort stop, FreezeType freezeType, DatabaseHandle databaseHandle) => FreezeResult.NotSupported;
            public FreezeResult FreezeCountersRangeAtTime(ushort start, ushort stop, DatabaseHandle databaseHandle, ulong time, uint interval) => FreezeResult.NotSupported;
            public bool SupportWriteAnalogDeadBands() => false;
            public void BeginWriteAnalogDeadBands() { }
            public void WriteAnalogDeadBand(ushort index, double deadBand) { }
            public void EndWriteAnalogDeadBands() { }
            public bool WriteStringAttr(byte set, byte variation, StringAttr attrType, string value) => false;
            public bool WriteFloatAttr(byte set, byte variation, FloatAttr attrType, float value) => false;
            public bool WriteDoubleAttr(byte set, byte variation, FloatAttr attrType, double value) => false;
            public bool WriteUintAttr(byte set, byte variation, UintAttr attrType, uint value) => false;
            public bool WriteIntAttr(byte set, byte variation, IntAttr attrType, int value) => false;
            public bool WriteOctetStringAttr(byte set, byte variation, OctetStringAttr attrType, ICollection<byte> value) => false;
            public bool WriteBitStringAttr(byte set, byte variation, BitStringAttr attrType, ICollection<byte> value) => false;
            public bool WriteTimeAttr(byte set, byte variation, TimeAttr attrType, ulong value) => false;
            public void BeginConfirm() { }
            public void EventCleared(ulong id) { }
            public void EndConfirm(BufferState state) { }
        }

        class OutstationInformation : IOutstationInformation
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

        class ControlHandler : IControlHandler
        {
            readonly TaskCompletionSource<Control> received = new TaskCompletionSource<Control>(TaskCreationOptions.RunContinuationsAsynchronously);

            public Task<Control> Received => received.Task;

            public void BeginFragment() { }
            public void EndFragment(DatabaseHandle database) { }
            public CommandStatus SelectG12v1(Group12Var1 value, ushort index, DatabaseHandle databaseHandle) => CommandStatus.NotSupported;

            public CommandStatus OperateG12v1(Group12Var1 value, ushort index, OperateType opType, DatabaseHandle databaseHandle)
            {
                received.TrySetResult(new Control { Index = index, OpType = value.Code.OpType, OperateType = opType, Count = value.Count, OnTime = value.OnTime, OffTime = value.OffTime });
                return CommandStatus.Success;
            }

            public CommandStatus SelectG41v1(int value, ushort index, DatabaseHandle databaseHandle) => CommandStatus.NotSupported;
            public CommandStatus OperateG41v1(int value, ushort index, OperateType opType, DatabaseHandle databaseHandle) => CommandStatus.NotSupported;
            public CommandStatus SelectG41v2(short value, ushort index, DatabaseHandle databaseHandle) => CommandStatus.NotSupported;
            public CommandStatus OperateG41v2(short value, ushort index, OperateType opType, DatabaseHandle databaseHandle) => CommandStatus.NotSupported;
            public CommandStatus SelectG41v3(float value, ushort index, DatabaseHandle databaseHandle) => CommandStatus.NotSupported;
            public CommandStatus OperateG41v3(float value, ushort index, OperateType opType, DatabaseHandle databaseHandle) => CommandStatus.NotSupported;
            public CommandStatus SelectG41v4(double value, ushort index, DatabaseHandle databaseHandle) => CommandStatus.NotSupported;
            public CommandStatus OperateG41v4(double value, ushort index, OperateType opType, DatabaseHandle databaseHandle) => CommandStatus.NotSupported;
        }

        // records binary input 0 and analog input 0; the read completes after the handler runs
        class ReadHandler : IReadHandler
        {
            readonly object mutex = new object();
            bool? binaryValue;
            double? analogValue;

            public bool? BinaryValue { get { lock (mutex) { return binaryValue; } } }
            public double? AnalogValue { get { lock (mutex) { return analogValue; } } }

            public void HandleBinaryInput(HeaderInfo info, ICollection<BinaryInput> values)
            {
                foreach (var value in values)
                {
                    if (value.Index == 0) { lock (mutex) { binaryValue = value.Value; } }
                }
            }

            public void HandleAnalogInput(HeaderInfo info, ICollection<AnalogInput> values)
            {
                foreach (var value in values)
                {
                    if (value.Index == 0) { lock (mutex) { analogValue = value.Value; } }
                }
            }

            public void BeginFragment(ReadType readType, ResponseHeader header) { }
            public void EndFragment(ReadType readType, ResponseHeader header) { }
            public void HandleDoubleBitBinaryInput(HeaderInfo info, ICollection<DoubleBitBinaryInput> values) { }
            public void HandleBinaryOutputStatus(HeaderInfo info, ICollection<BinaryOutputStatus> values) { }
            public void HandleCounter(HeaderInfo info, ICollection<Counter> values) { }
            public void HandleFrozenCounter(HeaderInfo info, ICollection<FrozenCounter> values) { }
            public void HandleFrozenAnalogInput(HeaderInfo info, ICollection<FrozenAnalogInput> values) { }
            public void HandleAnalogOutputStatus(HeaderInfo info, ICollection<AnalogOutputStatus> values) { }
            public void HandleBinaryOutputCommandEvent(HeaderInfo info, ICollection<BinaryOutputCommandEvent> values) { }
            public void HandleAnalogOutputCommandEvent(HeaderInfo info, ICollection<AnalogOutputCommandEvent> values) { }
            public void HandleUnsignedInteger(HeaderInfo info, ICollection<UnsignedInteger> values) { }
            public void HandleOctetString(HeaderInfo info, ICollection<OctetString> values) { }
            public void HandleAbsTime(HeaderInfo info, Timestamp time) { }
            public void HandleStringAttr(HeaderInfo info, StringAttr attr, byte set, byte variation, string value) { }
            public void HandleVariationListAttr(HeaderInfo info, VariationListAttr attr, byte set, byte variation, ICollection<AttrItem> value) { }
            public void HandleUintAttr(HeaderInfo info, UintAttr attr, byte set, byte variation, uint value) { }
            public void HandleBoolAttr(HeaderInfo info, BoolAttr attr, byte set, byte variation, bool value) { }
            public void HandleIntAttr(HeaderInfo info, IntAttr attr, byte set, byte variation, int value) { }
            public void HandleTimeAttr(HeaderInfo info, TimeAttr attr, byte set, byte variation, ulong value) { }
            public void HandleFloatAttr(HeaderInfo info, FloatAttr attr, byte set, byte variation, double value) { }
            public void HandleOctetStringAttr(HeaderInfo info, OctetStringAttr attr, byte set, byte variation, ICollection<byte> value) { }
            public void HandleBitStringAttr(HeaderInfo info, BitStringAttr attr, byte set, byte variation, ICollection<byte> value) { }
        }

        class AssociationInformation : IAssociationInformation
        {
            public void TaskStart(TaskType taskType, FunctionCode functionCode, byte seq) { }
            public void TaskSuccess(TaskType taskType, FunctionCode functionCode, byte seq) { }
            public void TaskFail(TaskType taskType, TaskError error) { }
            public void UnsolicitedResponse(bool isDuplicate, byte seq) { }
        }

        [TestMethod]
        [Timeout(60000)]
        public async Task MasterReadsAndOperatesOutstation()
        {
            var runtime = new Runtime(new RuntimeConfig { NumCoreThreads = 2 });
            try
            {
                var server = OutstationServer.CreateTcpServer(runtime, LinkErrorMode.Close, Endpoint);
                var controlHandler = new ControlHandler();
                server.AddOutstation(
                    new OutstationConfig(OutstationAddress, MasterAddress, EventBufferConfig.NoEvents()),
                    new OutstationApplication(),
                    new OutstationInformation(),
                    controlHandler,
                    dnp3.functional.ConnectionStateListener.create(state => { }),
                    AddressFilter.Any()
                ).Transaction(db =>
                {
                    var online = new Flags(Flag.Online);
                    db.AddBinaryInput(0, EventClass.None, new BinaryInputConfig());
                    db.AddAnalogInput(0, EventClass.None, new AnalogInputConfig());
                    db.UpdateBinaryInput(new BinaryInput(0, true, online, Timestamp.InvalidTimestamp()), UpdateOptions.DetectEvent());
                    db.UpdateAnalogInput(new AnalogInput(0, 42.0, online, Timestamp.InvalidTimestamp()), UpdateOptions.DetectEvent());
                });
                server.Bind();

                var connected = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
                var channel = MasterChannel.CreateTcpChannel(
                    runtime,
                    LinkErrorMode.Close,
                    new MasterChannelConfig(MasterAddress),
                    new EndpointList(Endpoint),
                    new ConnectStrategy(),
                    dnp3.functional.ClientStateListener.create(state =>
                    {
                        if (state == ClientState.Connected) connected.TrySetResult(true);
                    })
                );
                // no startup integrity poll or unsolicited setup, so only the test's own requests are made
                var readHandler = new ReadHandler();
                var association = channel.AddAssociation(
                    OutstationAddress,
                    new AssociationConfig(EventClasses.None(), EventClasses.None(), Classes.None(), EventClasses.None()),
                    readHandler,
                    dnp3.functional.AssociationHandler.create(() => UtcTimestamp.Invalid()),
                    new AssociationInformation()
                );
                channel.Enable();
                await connected.Task.WaitAsync(WaitTimeout);

                // a class 0 read returns the outstation's values
                await channel.Read(association, Request.ClassRequest(true, false, false, false)).WaitAsync(WaitTimeout);
                Assert.AreEqual(true, readHandler.BinaryValue);
                Assert.AreEqual(42.0, readHandler.AnalogValue);

                // a direct operate reaches the outstation's control handler
                var commands = new CommandSet();
                commands.AddG12V1U16(0, new Group12Var1(new ControlCode(TripCloseCode.Nul, false, OpType.LatchOn), 1, 1000, 2000));
                await channel.Operate(association, CommandMode.DirectOperate, commands).WaitAsync(WaitTimeout);
                var control = await controlHandler.Received.WaitAsync(WaitTimeout);
                Assert.AreEqual((ushort)0, control.Index);
                Assert.AreEqual(OpType.LatchOn, control.OpType);
                Assert.AreEqual(OperateType.DirectOperate, control.OperateType);
                Assert.AreEqual((byte)1, control.Count);
                Assert.AreEqual(1000u, control.OnTime);
                Assert.AreEqual(2000u, control.OffTime);

                channel.Shutdown();
                server.Shutdown();
            }
            finally
            {
                runtime.Shutdown();
            }
        }
    }
}
