package io.stepfunc.conformance.dnp3

import io.stepfunc.dnp3c.dnp3.app._
import io.stepfunc.dnp3c.dnp3.app.objects.AttributeValue
import io.stepfunc.dnp3c.dnp3.{AbnormalFlag, ActionResult, ActionTiming, CheckReporter, DeferredCheck, EventExpectation, InputKind, IntegrationPlugin, OutputKind, PluginReporter, PointKind, TimeQuality}
import io.stepfunc.dnp3c.protocol.parsing.UInt8
import io.stepfunc.dnp3.{AddressFilter, AnalogInput, AnalogOutputStatus, BinaryInput, BinaryOutputStatus, Counter, DoubleBit, DoubleBitBinaryInput, EventBufferConfig, FrozenCounter, LinkErrorMode, Outstation, Runtime, RuntimeConfig, OutstationServer}
import org.joou.UInteger
import org.joou.Unsigned.{uint, ushort}

import java.time.Duration
import scala.collection.mutable
import scala.reflect.ClassTag

class Dnp3IntegrationPlugin extends IntegrationPlugin {
  private var runtime: Runtime = _
  private var server: OutstationServer = _
  private var outstation: Outstation = _
  private var trackingDatabase: TrackingDatabase = _
  private var controlHandler: QueuedControlHandler = _
  private var config = StackConfig.Default
  private var eventBatch = 0
  private var app: CustomOutstationApplication = _
  private var coldRestartSupported = true


  override def startProcedure(reporter: PluginReporter): Unit = {
    config = StackConfig.Default
    coldRestartSupported = true
    startOutstation()
  }

  override def endProcedure(reporter: PluginReporter): Unit = {
    shutdown()
  }

  override def cyclePower(reporter: PluginReporter): Unit = {
    reporter.log("Cycling power")
    shutdown()
    startOutstation()
  }

  override def setLinkLayerConfirm(reporter: PluginReporter, value: Boolean): Unit = {
    if(value) {
      throw new RuntimeException("Confirmed link-layer is not supported by dnp3")
    }
  }

  override def setSelfAddressSupport(reporter: PluginReporter, value: Boolean): Unit = {
    config = config.copy(linkConfig = config.linkConfig.copy(selfAddressSupport = value))
    reporter.log(s"Self address support is set to $value")
    cyclePower(reporter)
  }

  override def checkOperate(reporter: CheckReporter, kind: OutputKind, index: Int): Option[DeferredCheck] = {
    kind match {
      case PointKind.BinaryOutputStatus => controlHandler.checkCrob(index)
      case PointKind.AnalogOutputStatus => controlHandler.checkAnalog(index)
    }
    reporter.log(s"${output(kind)} $index did operate")
    None
  }

  override def checkNotOperate(reporter: CheckReporter, kind: OutputKind): Option[DeferredCheck] = {
    kind match {
      case PointKind.BinaryOutputStatus => controlHandler.checkNoCrob()
      case PointKind.AnalogOutputStatus => controlHandler.checkNoAnalog()
    }
    reporter.log(s"${output(kind)} did NOT operate")
    None
  }

  private def output(kind: OutputKind): String = kind match {
    case PointKind.BinaryOutputStatus => "Binary output"
    case PointKind.AnalogOutputStatus => "Analog output"
  }

  override def generateClassEvents(reporter: PluginReporter, eventClass: EventClass): Unit = {
    eventClass match {
      case EventClass.Class1 => generateInputChanges(reporter, PointKind.BinaryInput)
      case EventClass.Class2 => generateInputChanges(reporter, PointKind.Counter)
      case EventClass.Class3 => generateInputChanges(reporter, PointKind.AnalogInput)
      case _ => {
        generateInputChanges(reporter, PointKind.BinaryInput)
        this.eventBatch = this.eventBatch - 1
        generateInputChanges(reporter, PointKind.Counter)
        this.eventBatch = this.eventBatch - 1
        generateInputChanges(reporter, PointKind.AnalogInput)
      }
    }
  }

  override def generateExactClassEvents(reporter: PluginReporter, eventClass: EventClass, numEvents: Int): Unit = {
    eventClass match {
      case EventClass.Class1 => generateExactInputEvents(reporter, PointKind.BinaryInput, numEvents)
      case EventClass.Class2 => generateExactInputEvents(reporter, PointKind.Counter, numEvents)
      case EventClass.Class3 => generateExactInputEvents(reporter, PointKind.AnalogInput, numEvents)
      case _ => throw new Exception(s"Cannot generate an exact number of $eventClass events")
    }
  }

  override def generateInputChanges(reporter: PluginReporter, kind: InputKind[?]): Unit = {
    this.eventBatch = this.eventBatch + 1
    0 to 9 foreach(idx => generateInputEvent(reporter, kind, idx))
  }

  override def generateExactInputEvents(reporter: PluginReporter, kind: InputKind[?], numEvents: Int): Unit = {
    this.eventBatch = this.eventBatch + 1
    1 to numEvents foreach(_ => generateInputEvent(reporter, kind, 0))
  }

  private def generateInputEvent(reporter: PluginReporter, kind: InputKind[?], idx: Int): Unit = kind match {
    case PointKind.BinaryInput => printBinaryInput(reporter, trackingDatabase.generateBinaryInputEvent(idx, this.eventBatch))
    case PointKind.DoubleBitInput => printDoubleBitBinaryInput(reporter, trackingDatabase.generateDoubleBitBinaryInputEvent(idx, this.eventBatch))
    case PointKind.Counter => printCounter(reporter, trackingDatabase.generateCounterEvent(idx, this.eventBatch))
    case PointKind.AnalogInput => printAnalogInput(reporter, trackingDatabase.generateAnalogInputEvent(idx, this.eventBatch))
  }

  override def generatePattern(reporter: PluginReporter, kind: InputKind[?]): Unit = {
    generateInputChanges(reporter, kind)
  }

  override def generateTimeEvent(reporter: PluginReporter): Unit = {
    this.eventBatch = this.eventBatch + 1
    val event = trackingDatabase.generateBinaryInputEvent(0, this.eventBatch)
    printBinaryInput(reporter, event)
  }

  // the quality the application gives the timestamps of events recorded now
  override def timeQuality(reporter: PluginReporter): Option[TimeQuality] =
    Some(if (app.isTimeValid) TimeQuality.Synchronized else TimeQuality.Unsynchronized)

  override def uninstall(reporter: PluginReporter, kind: PointKind): Unit = {
    val database = config.testDatabaseConfig
    val commands = config.commandHandlerConfig
    config = kind match {
      case PointKind.BinaryInput => config.copy(testDatabaseConfig = database.copy(disableBinaryInputs = true))
      case PointKind.DoubleBitInput => config.copy(testDatabaseConfig = database.copy(disableDoubleBitBinaryInputs = true))
      case PointKind.Counter => config.copy(testDatabaseConfig = database.copy(disableCounters = true))
      case PointKind.FrozenCounter => config.copy(testDatabaseConfig = database.copy(disableFrozenCounters = true))
      case PointKind.AnalogInput => config.copy(testDatabaseConfig = database.copy(disableAnalogInputs = true))
      case PointKind.BinaryOutputStatus => config.copy(testDatabaseConfig = database.copy(disableBinaryOutputStatus = true), commandHandlerConfig = commands.copy(disableBinaryOutput = true))
      case PointKind.AnalogOutputStatus => config.copy(testDatabaseConfig = database.copy(disableAnalogOutputStatus = true), commandHandlerConfig = commands.copy(disableAnalogOutput = true))
    }
    reporter.log(s"All ${kind.name} points were uninstalled")
    cyclePower(reporter)
  }

  // Actions 3.1r1 uses

  override def setColdRestartSupport(reporter: PluginReporter, enabled: Boolean): Unit = {
    coldRestartSupported = enabled
    app.coldRestartSupported = enabled
    reporter.log(s"Cold restart support is set to $enabled")
  }

  override def setAnalogInput(reporter: PluginReporter, index: Int, value: Double): Unit = {
    this.eventBatch = this.eventBatch + 1
    printAnalogInput(reporter, trackingDatabase.setAnalogInputValue(index, value, this.eventBatch))
  }

  override def generateExactFrozenCounterEvents(reporter: PluginReporter, numEvents: Int): Unit = {
    this.eventBatch = this.eventBatch + 1
    1 to numEvents foreach { _ =>
      val event = trackingDatabase.generateFrozenCounterEvent(0, this.eventBatch)
      reporter.log(s"Updated frozen counter ${event.idx}: value=${event.value.value}")
    }
  }

  // the analog input values go up by 100 an event from 0, so they stay below 32768
  override def generateExactSmallAnalogInputEvents(reporter: PluginReporter, numEvents: Int): Unit =
    generateExactInputEvents(reporter, PointKind.AnalogInput, numEvents)

  // the change of one binary input, the kind 3.1r1's 8.26 uses
  override def generateEventOn(reporter: PluginReporter, kind: PointKind, index: Int): Unit = kind match {
    case PointKind.BinaryInput =>
      this.eventBatch = this.eventBatch + 1
      printBinaryInput(reporter, trackingDatabase.generateBinaryInputEvent(index, this.eventBatch))
    case _ => throw new Exception(s"Events on one ${kind.name} aren't supported")
  }

  override def setFlagsNormal(reporter: PluginReporter, kind: PointKind): Unit = {
    trackingDatabase.setFlags(kind, None, 0x01)
    reporter.log(s"Every ${kind.name} point's flags are normal")
  }

  // COMM_LOST set on the first point, with ONLINE
  override def setFlagAbnormal(reporter: PluginReporter, kind: PointKind, indices: Seq[Int]): AbnormalFlag = {
    val index = indices.headOption.getOrElse(throw new Exception(s"No ${kind.name} point to set a flag of"))
    trackingDatabase.setFlags(kind, Some(Seq(index)), 0x05)
    reporter.log(s"${kind.name} $index has COMM_LOST set")
    AbnormalFlag(index, Some(0x05))
  }

  override def verifyFrozenCounterEvents(reporter: CheckReporter, points: Seq[GenIndexedPoint[CounterPoint]]): Option[DeferredCheck] = {
    verifyLatestEvents(reporter, points, "frozen counter", trackingDatabase.popFrozenCounterEvent, checkFrozenCounter)
    None
  }

  override def reinstallPoints(reporter: PluginReporter): Unit = {
    config = config.copy(
      testDatabaseConfig = config.testDatabaseConfig.copy(disableBinaryInputs = false, disableDoubleBitBinaryInputs = false, disableCounters = false, disableFrozenCounters = false, disableAnalogInputs = false, disableBinaryOutputStatus = false, disableAnalogOutputStatus = false),
      commandHandlerConfig = CommandHandlerConfig(disableBinaryOutput = false, disableAnalogOutput = false)
    )
    reporter.log("All points were reinstalled")
    cyclePower(reporter)
  }

  override def enableRemoteSupervisoryControl(reporter: PluginReporter): Unit = {
    config = config.copy(testDatabaseConfig =
      config.testDatabaseConfig.copy(isGlobalLocalControl = false, localControlIndex = None))
    trackingDatabase.clearLocalControl()
    reporter.log("Every binary output is under remote control")
  }

  override def setGlobalRemoteSupervisoryControl(reporter: PluginReporter): Unit = {
    config = config.copy(testDatabaseConfig = config.testDatabaseConfig.copy(isGlobalLocalControl = true))
    trackingDatabase.setLocalControl(0 to 19)
    reporter.log("Global remote supervisory control was enabled")
  }

  override def setIndividualRemoteSupervisoryControl(reporter: PluginReporter, index: Int): Unit = {
    config = config.copy(testDatabaseConfig = config.testDatabaseConfig.copy(localControlIndex = Some(index)))
    trackingDatabase.setLocalControl(Seq(index))
    reporter.log(s"Binary output $index is under local control")
  }

  override def enableUnsolicitedResponse(reporter: PluginReporter, enabled: Boolean): Unit = {
    config = config.copy(unsolicitedResponseConfig = config.unsolicitedResponseConfig.copy(enabled))
    reporter.log("Unsolicited responses were enabled")
    cyclePower(reporter)
  }

  override def setUnsolicitedResponseTimeout(reporter: PluginReporter, timeoutMs: Int): Unit = {
    // the outstation has one confirm timeout, for solicited and unsolicited responses
    config = config.copy(outstationConfig = config.outstationConfig.copy(confirmTimeoutMs = timeoutMs))
    reporter.log(f"Confirm timeout, for solicited and unsolicited responses, was set to $timeoutMs ms")
    cyclePower(reporter)
  }

  override def setMaxUnsolicitedRetries(reporter: PluginReporter, numRetries: Option[Int]): Unit = {
    val strNumRetries = numRetries match {
      case Some(e) => e.toString
      case None => "Infinite"
    }
    config = config.copy(unsolicitedResponseConfig = config.unsolicitedResponseConfig.copy(maxNumRetries = numRetries))
    reporter.log(s"Max unsolicited response retries was set to $strNumRetries")
    cyclePower(reporter)
  }

  override def setMasterAddress(reporter: PluginReporter, address: Int): Unit = {
    config = config.copy(linkConfig = config.linkConfig.copy(destination = address))
    reporter.log(f"Master address was set to $address")
    cyclePower(reporter)
  }

  override def generateUnsolicitedEvents(reporter: PluginReporter, timing: ActionTiming): ActionResult = {
    generateExactInputEvents(reporter, PointKind.BinaryInput, 1)
    ActionResult.Performed
  }

  override def setMaxFragmentSize(reporter: PluginReporter, maxFragmentSize: Int): Unit = {
    config = config.copy(outstationConfig = config.outstationConfig.copy(fragmentSize = maxFragmentSize))
    reporter.log(f"Max fragment size was set to $maxFragmentSize")
    cyclePower(reporter)
  }

  override def generateMultiFragmentResponse(reporter: PluginReporter): Unit = {
    generateExactInputEvents(reporter, PointKind.BinaryInput, 200)
    generateExactInputEvents(reporter, PointKind.DoubleBitInput, 200)
    generateExactInputEvents(reporter, PointKind.Counter, 200)
    generateExactInputEvents(reporter, PointKind.AnalogInput, 200)
  }

  override def recordCurrentCounterValues(reporter: PluginReporter): Unit = {
    this.trackingDatabase.recordCurrentCounterValues()

    reporter.log("Recorded values:")
    this.trackingDatabase.getRecordedCounters.foreach(p => {
      printCounter(reporter, p)
    })
  }

  override def verifyAllStaticCurrentValues(reporter: CheckReporter, isClass0: Boolean, points: Seq[GenIndexedPoint[MeasurementPoint]]): Option[DeferredCheck] = {
    val expectedPoints = this.trackingDatabase.getAllPoints(isClass0).to(mutable.Set)
    val numExpectedPoints = expectedPoints.size

    def findPoint[T](idx: Int)(implicit tag: ClassTag[T]): TypedEvent[T] = {
      val point = expectedPoints.find {
        case event @ TypedEvent(_: T) if event.idx == idx => true
        case _ => false
      }

      point match {
        case Some(p) => {
          expectedPoints.remove(p)
          p.asInstanceOf[TypedEvent[T]]
        }
        case None => throw new Exception("Received unexpected point")
      }
    }

    points.foreach(receivedPoint => {
      receivedPoint.point match {
        case _: BinaryPoint => {
          val expectedPoint = findPoint[BinaryInput](receivedPoint.idx)
          checkBinaryInput(expectedPoint, receivedPoint.asInstanceOf[GenIndexedPoint[BinaryPoint]])
        }
        case _: DoubleBitPoint => {
          val expectedPoint = findPoint[DoubleBitBinaryInput](receivedPoint.idx)
          checkDoubleBitBinaryInput(expectedPoint, receivedPoint.asInstanceOf[GenIndexedPoint[DoubleBitPoint]])
        }
        case p: CounterPoint if !p.isFrozen => {
          val expectedPoint = findPoint[Counter](receivedPoint.idx)
          checkCounter(expectedPoint, receivedPoint.asInstanceOf[GenIndexedPoint[CounterPoint]])
        }
        case p: CounterPoint if p.isFrozen => {
          val expectedPoint = findPoint[FrozenCounter](receivedPoint.idx)
          checkFrozenCounter(expectedPoint, receivedPoint.asInstanceOf[GenIndexedPoint[CounterPoint]])
        }
        case _: AnalogPoint => {
          val expectedPoint = findPoint[AnalogInput](receivedPoint.idx)
          checkAnalogInput(expectedPoint, receivedPoint.asInstanceOf[GenIndexedPoint[AnalogPoint]])
        }
        case _: BinaryOutputStatusPoint => {
          val expectedPoint = findPoint[BinaryOutputStatus](receivedPoint.idx)
          checkBinaryOutput(expectedPoint, receivedPoint.asInstanceOf[GenIndexedPoint[BinaryOutputStatusPoint]])
        }
        case _: AnalogOutputStatusPoint => {
          val expectedPoint = findPoint[AnalogOutputStatus](receivedPoint.idx)
          checkAnalogOutput(expectedPoint, receivedPoint.asInstanceOf[GenIndexedPoint[AnalogOutputStatusPoint]])
        }
        case _ => throw new Exception("Unknown point type")
      }
    })

    if(expectedPoints.nonEmpty) throw new Exception("DUT did not report all the points")

    reporter.log(f"Verified $numExpectedPoints point(s)")
    None
  }

  override def verifyClassEvents(reporter: CheckReporter, eventClass: EventClass, expectation: EventExpectation, points: Seq[GenIndexedPoint[MeasurementPoint]]): Option[DeferredCheck] = expectation match {
    case EventExpectation.LatestBatch => verifyLatestClassEvents(reporter, eventClass, points)
    case EventExpectation.RemainingLatestBatch => verifyRestClassEvents(reporter, eventClass, points)
    case EventExpectation.AllGeneratedBatches => verifyAllClassEvents(reporter, eventClass, points)
    case EventExpectation.ReturnedEventsOnly => verifyReturnedClassEvents(reporter, points)
    case EventExpectation.CurrentValuesWithOptionalEarlier => throw new Exception(s"Unsupported check of $eventClass events: $expectation")
  }

  // each returned event matches an event generated for its point; there may be none
  private def verifyReturnedClassEvents(reporter: CheckReporter, points: Seq[GenIndexedPoint[MeasurementPoint]]): Option[DeferredCheck] = {
    val generated = trackingDatabase.getAllEvents(BatchSpecifier.All)
    points.foreach(received => {
      if (!generated.exists(expected => scala.util.Try(checkClassEvent(expected, received)).isSuccess)) {
        throw new Exception(s"Event at index ${received.idx} matches no event generated")
      }
    })
    reporter.log(f"Verified ${points.size} returned event(s)")
    None
  }

  // checks a received event against an expected one of whatever type
  private def checkClassEvent(expectedEvent: Event, receivedEvent: GenIndexedPoint[MeasurementPoint]): Unit = {
    if (generatedKind(expectedEvent) != receivedKind(receivedEvent)) throw new Exception("Received unexpected event type")
    def typed[T <: MeasurementPoint](implicit tag: ClassTag[T]): GenIndexedPoint[T] = receivedEvent.point match {
      case _: T => receivedEvent.asInstanceOf[GenIndexedPoint[T]]
      case _ => throw new Exception("Received unexpected event type")
    }
    expectedEvent match {
      case TypedEvent(_: BinaryInput) => checkBinaryInput(expectedEvent.asInstanceOf[TypedEvent[BinaryInput]], typed[BinaryPoint])
      case TypedEvent(_: DoubleBitBinaryInput) => checkDoubleBitBinaryInput(expectedEvent.asInstanceOf[TypedEvent[DoubleBitBinaryInput]], typed[DoubleBitPoint])
      case TypedEvent(_: Counter) => checkCounter(expectedEvent.asInstanceOf[TypedEvent[Counter]], typed[CounterPoint])
      case TypedEvent(_: FrozenCounter) => checkFrozenCounter(expectedEvent.asInstanceOf[TypedEvent[FrozenCounter]], typed[CounterPoint])
      case TypedEvent(_: AnalogInput) => checkAnalogInput(expectedEvent.asInstanceOf[TypedEvent[AnalogInput]], typed[AnalogPoint])
      case TypedEvent(_: BinaryOutputStatus) => checkBinaryOutput(expectedEvent.asInstanceOf[TypedEvent[BinaryOutputStatus]], typed[BinaryOutputStatusPoint])
      case TypedEvent(_: AnalogOutputStatus) => checkAnalogOutput(expectedEvent.asInstanceOf[TypedEvent[AnalogOutputStatus]], typed[AnalogOutputStatusPoint])
    }
  }

  private def verifyLatestClassEvents(reporter: CheckReporter, eventClass: EventClass, points: Seq[GenIndexedPoint[MeasurementPoint]]): Option[DeferredCheck] = {
    trackingDatabase.resetHandledEvents()

    points.foreach(receivedEvent => {
      val expectedEvent = trackingDatabase.popEvent(BatchSpecifier.Specific(this.eventBatch), eventClass).getOrElse(throw new Exception("Unexpected event"))
      checkClassEvent(expectedEvent, receivedEvent)
    })

    if (trackingDatabase.popEvent(BatchSpecifier.Specific(this.eventBatch), eventClass).isDefined) throw new Exception("Missing event")

    reporter.log(f"Verified ${points.size} $eventClass event(s)")
    None
  }

  override def verifyFirstClassEvent(reporter: CheckReporter, eventClass: EventClass, point: GenIndexedPoint[MeasurementPoint]): Option[DeferredCheck] = {

    val expectedEvent = trackingDatabase.popEvent(BatchSpecifier.Specific(this.eventBatch), eventClass).getOrElse(throw new Exception("Unexpected event"))
    checkClassEvent(expectedEvent, point)

    reporter.log(f"Verified first $eventClass event")
    None
  }

  private def verifyRestClassEvents(reporter: CheckReporter, eventClass: EventClass, points: Seq[GenIndexedPoint[MeasurementPoint]]): Option[DeferredCheck] = {

    points.foreach(receivedEvent => {
      val expectedEvent = trackingDatabase.popEvent(BatchSpecifier.Specific(this.eventBatch), eventClass).getOrElse(throw new Exception("Unexpected event"))
      checkClassEvent(expectedEvent, receivedEvent)
    })

    if (trackingDatabase.popEvent(BatchSpecifier.Specific(this.eventBatch), eventClass).isDefined) throw new Exception("Missing event")

    reporter.log(f"Verified ${points.size} $eventClass event(s)")
    None
  }

  private def verifyAllClassEvents(reporter: CheckReporter, eventClass: EventClass, points: Seq[GenIndexedPoint[MeasurementPoint]]): Option[DeferredCheck] = {
    trackingDatabase.resetHandledEvents()

    points.foreach(receivedEvent => {
      val expectedEvent = trackingDatabase.popEvent(BatchSpecifier.All, eventClass).getOrElse(throw new Exception("Unexpected event"))
      checkClassEvent(expectedEvent, receivedEvent)
    })

    if (trackingDatabase.popEvent(BatchSpecifier.All, eventClass).isDefined) throw new Exception("Missing event")

    reporter.log(f"Verified ${points.size} $eventClass event(s)")
    None
  }

  override def verifyInputCurrentValues[P <: MeasurementPoint](reporter: CheckReporter, kind: InputKind[P], points: Seq[GenIndexedPoint[P]]): Option[DeferredCheck] = {
    kind match {
      case PointKind.BinaryInput => verifyAllCurrentStatus(reporter, points, "binary input", trackingDatabase.getAllBinaryInputs, checkBinaryInput)
      case PointKind.DoubleBitInput => verifyAllCurrentStatus(reporter, points, "double-bit binary input", trackingDatabase.getAllDoubleBitBinaryInputs, checkDoubleBitBinaryInput)
      case PointKind.Counter => verifyAllCurrentStatus(reporter, points, "counter", trackingDatabase.getAllCounters, checkCounter)
      case PointKind.AnalogInput => verifyAllCurrentStatus(reporter, points, "analog input", trackingDatabase.getAllAnalogInputs, checkAnalogInput)
    }
    None
  }

  override def verifyRecordedCounterValues(reporter: CheckReporter, points: Seq[GenIndexedPoint[CounterPoint]]): Option[DeferredCheck] = {
    verifyAllCurrentStatus(reporter, points, "counter", trackingDatabase.getRecordedCounters, checkCounter)
    None
  }

  override def verifyInputEvents[P <: MeasurementPoint](reporter: CheckReporter, kind: InputKind[P], expectation: EventExpectation, points: Seq[GenIndexedPoint[P]]): Option[DeferredCheck] = {
    kind match {
      case PointKind.BinaryInput => verifyEvents(reporter, expectation, points, "binary input", trackingDatabase.popBinaryEvent, checkBinaryInput)
      case PointKind.DoubleBitInput => verifyEvents(reporter, expectation, points, "double-bit binary input", trackingDatabase.popDoubleBitBinaryEvent, checkDoubleBitBinaryInput)
      case PointKind.Counter if expectation == EventExpectation.CurrentValuesWithOptionalEarlier =>
        verifyCurrentValues(reporter, points, "counter", trackingDatabase.getAllCounterEvents(BatchSpecifier.All), checkCounter)
      case PointKind.AnalogInput if expectation == EventExpectation.CurrentValuesWithOptionalEarlier =>
        verifyCurrentValues(reporter, points, "analog input", trackingDatabase.getAllAnalogInputEvents(BatchSpecifier.All), checkAnalogInput)
      case PointKind.Counter => verifyEvents(reporter, expectation, points, "counter", trackingDatabase.popCounterEvent, checkCounter)
      case PointKind.AnalogInput => verifyEvents(reporter, expectation, points, "analog input", trackingDatabase.popAnalogInputEvent, checkAnalogInput)
    }
    None
  }

  // for each point changed in the latest batch, its last event reports the current value; events before it report
  // values the point had earlier
  private def verifyCurrentValues[ReceivedT <: MeasurementPoint, ExpectedT](reporter: PluginReporter, points: Seq[GenIndexedPoint[ReceivedT]], name: String, history: Seq[TypedEvent[ExpectedT]], checkMethod: (TypedEvent[ExpectedT], GenIndexedPoint[ReceivedT]) => Unit): Unit = {
    val current = history.filter(_.batch == this.eventBatch).groupBy(_.idx).view.mapValues(_.last).toMap
    val received = points.groupBy(_.idx)
    if (received.keySet != current.keySet) {
      throw new Exception(s"Expected $name events for points ${current.keys.toList.sorted.mkString(", ")}, received ${received.keys.toList.sorted.mkString(", ")}")
    }
    current.foreach { case (idx, expected) =>
      checkMethod(expected, received(idx).last)
      val earlier = history.filter(e => e.idx == idx && (e ne expected))
      received(idx).init.foreach(point => {
        if (!earlier.exists(e => scala.util.Try(checkMethod(e, point)).isSuccess)) throw new Exception(s"$name $idx reported a value it never had")
      })
    }
    reporter.log(f"Verified the current value of ${current.size} $name(s)")
  }

  override def verifyFirstInputEvent[P <: MeasurementPoint](reporter: CheckReporter, kind: InputKind[P], point: GenIndexedPoint[P]): Option[DeferredCheck] = {
    kind match {
      case PointKind.BinaryInput => verifyFirstEvent(reporter, point, "binary input", trackingDatabase.popBinaryEvent, checkBinaryInput)
      case PointKind.DoubleBitInput => verifyFirstEvent(reporter, point, "double-bit binary input", trackingDatabase.popDoubleBitBinaryEvent, checkDoubleBitBinaryInput)
      case PointKind.Counter => verifyFirstEvent(reporter, point, "counter", trackingDatabase.popCounterEvent, checkCounter)
      case PointKind.AnalogInput => verifyFirstEvent(reporter, point, "analog input", trackingDatabase.popAnalogInputEvent, checkAnalogInput)
    }
    None
  }

  private def verifyEvents[ReceivedT <: MeasurementPoint, ExpectedT](reporter: PluginReporter, expectation: EventExpectation, points: Seq[GenIndexedPoint[ReceivedT]], name: String, popMethod: BatchSpecifier => Option[TypedEvent[ExpectedT]], checkMethod: (TypedEvent[ExpectedT], GenIndexedPoint[ReceivedT]) => Unit): Unit = expectation match {
    case EventExpectation.LatestBatch => verifyLatestEvents(reporter, points, name, popMethod, checkMethod)
    case EventExpectation.RemainingLatestBatch => verifyRestEvents(reporter, points, name, popMethod, checkMethod)
    case EventExpectation.AllGeneratedBatches => verifyAllEvents(reporter, points, name, popMethod, checkMethod)
    case _ => throw new Exception(s"Unsupported check of $name events: $expectation")
  }

  override def verifyPolledDataDoNotRepeatUnsolicitedData(reporter: CheckReporter, unsolicitedResponsePoints: Seq[GenIndexedPoint[MeasurementPoint]], polledResponsePoints: Seq[GenIndexedPoint[MeasurementPoint]]): Option[DeferredCheck] = {
    verifyLatestClassEvents(reporter, EventClass.All, unsolicitedResponsePoints)
    // the polled response may carry newer events, but none the unsolicited response already reported
    polledResponsePoints.find(polled => unsolicitedResponsePoints.exists(sameEvent(_, polled))).foreach(repeated =>
      throw new Exception(s"The polled response repeats the event at index ${repeated.idx}"))
    None
  }

  override def verifyResponseRetainsEvents(reporter: CheckReporter, previousResponsePoints: Seq[GenIndexedPoint[MeasurementPoint]], currentResponsePoints: Seq[GenIndexedPoint[MeasurementPoint]]): Option[DeferredCheck] = {
    // every event is one that was generated, and each earlier event is still there or replaced by its point's current
    // value (8.11.2.5.3 step 5: "the data may be updated to reflect current values")
    val generated = trackingDatabase.getAllEvents(BatchSpecifier.All)
    def matches(expected: Event, received: GenIndexedPoint[MeasurementPoint]) =
      scala.util.Try(checkClassEvent(expected, received)).isSuccess
    currentResponsePoints.foreach(current =>
      if (!generated.exists(matches(_, current))) throw new Exception(s"The event at index ${current.idx} matches no event generated"))
    previousResponsePoints.foreach(previous => {
      val currentValue = generated.filter(e => e.idx == previous.idx && generatedKind(e) == receivedKind(previous)).lastOption
      val retained = currentResponsePoints.exists(sameEvent(previous, _)) ||
        currentResponsePoints.exists(current => receivedKind(current) == receivedKind(previous) && currentValue.exists(matches(_, current)))
      if (!retained) throw new Exception(s"The response no longer reports the event at index ${previous.idx}, or its current value")
    })
    reporter.log(s"Verified that the response retains the ${previousResponsePoints.size} earlier event(s)")
    None
  }

  // a point's kind: its class, and for counters whether it's frozen
  private def receivedKind(point: GenIndexedPoint[MeasurementPoint]): (Class[?], Boolean) = point.point match {
    case counter: CounterPoint => (classOf[CounterPoint], counter.isFrozen)
    case other => (other.getClass, false)
  }

  // the kind of the points a generated event is reported as
  private def generatedKind(event: Event): (Class[?], Boolean) = event match {
    case TypedEvent(_: BinaryInput) => (classOf[BinaryPoint], false)
    case TypedEvent(_: DoubleBitBinaryInput) => (classOf[DoubleBitPoint], false)
    case TypedEvent(_: Counter) => (classOf[CounterPoint], false)
    case TypedEvent(_: FrozenCounter) => (classOf[CounterPoint], true)
    case TypedEvent(_: AnalogInput) => (classOf[AnalogPoint], false)
    case TypedEvent(_: BinaryOutputStatus) => (classOf[BinaryOutputStatusPoint], false)
    case TypedEvent(_: AnalogOutputStatus) => (classOf[AnalogOutputStatusPoint], false)
  }

  // the same event, however its time was encoded (absolute, or relative to a CTO)
  private def sameEvent(a: GenIndexedPoint[MeasurementPoint], b: GenIndexedPoint[MeasurementPoint]): Boolean = {
    def resolved(p: MeasurementPoint) = p.copy_with_timestamp(p.timestamp.map(_.copy(isRelative = false)))
    a.idx == b.idx && resolved(a.point) == resolved(b.point)
  }

  override def generateNotEnoughUnsolicitedEvents(reporter: PluginReporter): Unit = {
    throw new Exception("OpenDNP3 generates unsolicited responses on every change")
  }

  // ========================
  // Generic helper functions
  // ========================

  private def verifyAllCurrentStatus[ReceivedT <: MeasurementPoint, ExpectedT](reporter: PluginReporter, points: Seq[GenIndexedPoint[ReceivedT]], name: String, expectedPointsSeq: Seq[TypedEvent[ExpectedT]], checkMethod: (TypedEvent[ExpectedT], GenIndexedPoint[ReceivedT]) => Unit): Unit = {
    val expectedPoints = mutable.Map(expectedPointsSeq.map(e => e.idx -> e): _*)
    val numExpectedPoints = expectedPoints.size

    points.foreach(actualPoint => {
      expectedPoints.get(actualPoint.idx) match {
        case Some(expectedPoint) => {
          expectedPoints.remove(actualPoint.idx)
          checkMethod(expectedPoint, actualPoint)
        }
        case None => throw new Exception(s"Unexpected $name")
      }
    })

    if (expectedPoints.nonEmpty) throw new Exception(s"Missing $name point")

    reporter.log(f"Verified $numExpectedPoints $name point(s)")
  }

  private def verifyLatestEvents[ReceivedT <: MeasurementPoint, ExpectedT](reporter: PluginReporter, points: Seq[GenIndexedPoint[ReceivedT]], name: String, popMethod: BatchSpecifier => Option[TypedEvent[ExpectedT]], checkMethod: (TypedEvent[ExpectedT], GenIndexedPoint[ReceivedT]) => Unit): Unit = {
    trackingDatabase.resetHandledEvents()
    points.foreach(point => {
      popMethod(BatchSpecifier.Specific(this.eventBatch)) match {
        case Some(event) => checkMethod(event, point)
        case None => throw new Exception(s"Unexpected $name event")
      }
    })

    if (popMethod(BatchSpecifier.Specific(this.eventBatch)).isDefined) throw new Exception(s"Missing $name event")

    reporter.log(f"Verified ${points.size} $name point event(s)")
  }

  private def verifyFirstEvent[ReceivedT <: MeasurementPoint, ExpectedT](reporter: PluginReporter, point: GenIndexedPoint[ReceivedT], name: String, popMethod: BatchSpecifier => Option[TypedEvent[ExpectedT]], checkMethod: (TypedEvent[ExpectedT], GenIndexedPoint[ReceivedT]) => Unit): Unit = {
    popMethod(BatchSpecifier.Specific(this.eventBatch)) match {
      case Some(event) => checkMethod(event, point)
      case None => throw new Exception(s"Unexpected $name event")
    }

    reporter.log(f"Verified first $name point event")
  }

  private def verifyRestEvents[ReceivedT <: MeasurementPoint, ExpectedT](reporter: PluginReporter, points: Seq[GenIndexedPoint[ReceivedT]], name: String, popMethod: BatchSpecifier => Option[TypedEvent[ExpectedT]], checkMethod: (TypedEvent[ExpectedT], GenIndexedPoint[ReceivedT]) => Unit): Unit = {
    points.foreach(point => {
      popMethod(BatchSpecifier.Specific(this.eventBatch)) match {
        case Some(event) => checkMethod(event, point)
        case None => throw new Exception(s"Unexpected $name event")
      }
    })

    if (popMethod(BatchSpecifier.Specific(this.eventBatch)).isDefined) throw new Exception(s"Missing $name input event")

    reporter.log(f"Verified ${points.size} $name point event(s)")
  }

  private def verifyAllEvents[ReceivedT <: MeasurementPoint, ExpectedT](reporter: PluginReporter, points: Seq[GenIndexedPoint[ReceivedT]], name: String, popMethod: BatchSpecifier => Option[TypedEvent[ExpectedT]], checkMethod: (TypedEvent[ExpectedT], GenIndexedPoint[ReceivedT]) => Unit): Unit = {
    trackingDatabase.resetHandledEvents()
    points.foreach(point => {
      popMethod(BatchSpecifier.All) match {
        case Some(event) => checkMethod(event, point)
        case None => throw new Exception(s"Unexpected $name input event")
      }
    })

    if (popMethod(BatchSpecifier.All).isDefined) throw new Exception(s"Missing $name event")

    reporter.log(f"Verified ${points.size} $name point event(s)")
  }

  // ========================
  // Verify individual points
  // ========================

  private def printBinaryInput(reporter: PluginReporter, event: TypedEvent[BinaryInput]): Unit = {
    reporter.log(f"Updated BI ${event.idx}: value=${event.value.value}, flags=${UInt8.fromByte(event.value.flags.value.byteValue()).value.toHexString}, timestamp=${event.value.time.value} (${event.value.time.quality})")
  }

  private def checkBinaryInput(expectedValue: TypedEvent[BinaryInput], receivedValue: GenIndexedPoint[BinaryPoint]): Unit = {
    // Check index
    if (expectedValue.idx != receivedValue.idx) throw new Exception("Unknown binary point event reported")

    // Check value
    if (expectedValue.value.value != receivedValue.point.value) throw new Exception("Binary did not report proper value")

    // Check flags (if present)
    receivedValue.point.flags match {
      case Some(flags) => if (flags != expectedValue.value.flags.value.byteValue()) throw new Exception("Binary did not report proper flags")
      case None =>
    }

    // Check timestamp (if present)
    receivedValue.point.timestamp match {
      case Some(timestamp) => if (timestamp.value != expectedValue.value.time.value.longValue()) throw new Exception("Binary did not report proper timestamp")
      case None =>
    }
  }

  private def printDoubleBitBinaryInput(reporter: PluginReporter, event: TypedEvent[DoubleBitBinaryInput]): Unit = {
    reporter.log(f"Updated DBBI ${event.idx}: value=${event.value.value}, flags=${UInt8.fromByte(event.value.flags.value.byteValue()).value.toHexString}, timestamp=${event.value.time.value} (${event.value.time.quality})")
  }

  private def checkDoubleBitBinaryInput(expectedValue: TypedEvent[DoubleBitBinaryInput], receivedValue: GenIndexedPoint[DoubleBitPoint]): Unit = {
    // Check index
    if (expectedValue.idx != receivedValue.idx) throw new Exception("Unknown double-bit binary point event reported")

    // Check value
    if ((expectedValue.value.value == DoubleBit.DETERMINED_OFF && receivedValue.point.value != io.stepfunc.dnp3c.dnp3.app.DoubleBit.DeterminedOff) ||
      (expectedValue.value.value == DoubleBit.DETERMINED_ON && receivedValue.point.value != io.stepfunc.dnp3c.dnp3.app.DoubleBit.DeterminedOn) ||
      (expectedValue.value.value == DoubleBit.INDETERMINATE && receivedValue.point.value != io.stepfunc.dnp3c.dnp3.app.DoubleBit.Indeterminate) ||
      (expectedValue.value.value == DoubleBit.INTERMEDIATE && receivedValue.point.value != io.stepfunc.dnp3c.dnp3.app.DoubleBit.Intermediate)) {
      throw new Exception(f"Double-bit binary ${receivedValue.idx} did not report proper value")
    }

    // Check flags (if present)
    receivedValue.point.flags match {
      case Some(flags) => if (flags != expectedValue.value.flags.value.byteValue()) throw new Exception("Double-bit binary did not report proper flags")
      case None =>
    }

    // Check timestamp (if present)
    receivedValue.point.timestamp match {
      case Some(timestamp) => if (timestamp.value != expectedValue.value.time.value.longValue()) throw new Exception("Double-bit binary did not report proper timestamp")
      case None =>
    }
  }

  private def printCounter(reporter: PluginReporter, event: TypedEvent[Counter]): Unit = {
    reporter.log(f"Updated Counter ${event.idx}: value=${event.value.value}, flags=${UInt8.fromByte(event.value.flags.value.byteValue()).value.toHexString}, timestamp=${event.value.time.value} (${event.value.time.quality})")
  }

  private def checkCounter(expectedValue: TypedEvent[Counter], receivedValue: GenIndexedPoint[CounterPoint]): Unit = {
    // Check index
    if (expectedValue.idx != receivedValue.idx) throw new Exception("Unknown counter point reported")

    // Check value
    if (expectedValue.value.value.longValue() != receivedValue.point.value) throw new Exception("Counter did not report proper value")

    // Check flags (if present)
    receivedValue.point.flags match {
      case Some(flags) => if (flags != expectedValue.value.flags.value.byteValue()) throw new Exception("Counter did not report proper flags")
      case None =>
    }

    // Check timestamp (if present)
    receivedValue.point.timestamp match {
      case Some(timestamp) => if (timestamp.value != expectedValue.value.time.value.longValue()) throw new Exception("Counter did not report proper timestamp")
      case None =>
    }
  }

  private def checkFrozenCounter(expectedValue: TypedEvent[FrozenCounter], receivedValue: GenIndexedPoint[CounterPoint]): Unit = {
    // Check index
    if (expectedValue.idx != receivedValue.idx) throw new Exception("Unknown frozen counter point reported")

    // Check value
    if (expectedValue.value.value.longValue() != receivedValue.point.value) throw new Exception("Frozen counter did not report proper value")

    // Check flags (if present)
    receivedValue.point.flags match {
      case Some(flags) => if (flags != expectedValue.value.flags.value.byteValue()) throw new Exception("Frozen counter did not report proper flags")
      case None =>
    }

    // Check timestamp (if present)
    receivedValue.point.timestamp match {
      case Some(timestamp) => if (timestamp.value != expectedValue.value.time.value.longValue()) throw new Exception("Frozen counter did not report proper timestamp")
      case None =>
    }
  }

  private def printAnalogInput(reporter: PluginReporter, event: TypedEvent[AnalogInput]): Unit = {
    reporter.log(f"Updated AI ${event.idx}: value=${event.value.value}, flags=${UInt8.fromByte(event.value.flags.value.byteValue()).value.toHexString}, timestamp=${event.value.time.value} (${event.value.time.quality})")
  }

  private def checkAnalogInput(expectedValue: TypedEvent[AnalogInput], receivedValue: GenIndexedPoint[AnalogPoint]): Unit = {
    // Check index
    if (expectedValue.idx != receivedValue.idx) throw new Exception("Unknown analog input point reported")

    // Check value
    if (expectedValue.value.value != receivedValue.point.value) throw new Exception("Analog input did not report proper value")

    // Check flags (if present)
    receivedValue.point.flags match {
      case Some(flags) => if (flags != expectedValue.value.flags.value.byteValue()) throw new Exception("Analog input did not report proper flags")
      case None =>
    }

    // Check timestamp (if present)
    receivedValue.point.timestamp match {
      case Some(timestamp) => if (timestamp.value != expectedValue.value.time.value.longValue()) throw new Exception("Analog input did not report proper timestamp")
      case None =>
    }
  }

  private def checkBinaryOutput(expectedValue: TypedEvent[BinaryOutputStatus], receivedValue: GenIndexedPoint[BinaryOutputStatusPoint]): Unit = {
    // Check index
    if (expectedValue.idx != receivedValue.idx) throw new Exception("Unknown binary output point reported")

    // Check value
    if (expectedValue.value.value != receivedValue.point.value) throw new Exception("Binary output did not report proper value")

    // Check flags (if present)
    receivedValue.point.flags match {
      case Some(flags) => if (flags != expectedValue.value.flags.value.byteValue()) throw new Exception("Binary output did not report proper flags")
      case None =>
    }

    // Check timestamp (if present)
    receivedValue.point.timestamp match {
      case Some(timestamp) => if (timestamp.value != expectedValue.value.time.value.longValue()) throw new Exception("Binary output did not report proper timestamp")
      case None =>
    }
  }

  private def checkAnalogOutput(expectedValue: TypedEvent[AnalogOutputStatus], receivedValue: GenIndexedPoint[AnalogOutputStatusPoint]): Unit = {
    // Check index
    if (expectedValue.idx != receivedValue.idx) throw new Exception("Unknown analog output point reported")

    // Check value
    if (expectedValue.value.value != receivedValue.point.value) throw new Exception("Analog output did not report proper value")

    // Check flags (if present)
    receivedValue.point.flags match {
      case Some(flags) => if (flags != expectedValue.value.flags.value.byteValue()) throw new Exception("Analog output did not report proper flags")
      case None =>
    }

    // Check timestamp (if present)
    receivedValue.point.timestamp match {
      case Some(timestamp) => if (timestamp.value != expectedValue.value.time.value.longValue()) throw new Exception("Analog output did not report proper timestamp")
      case None =>
    }
  }

  override def checkAnalogOutputTolerance(reporter: CheckReporter, expectedValue: Double, actualValue: Double): Option[DeferredCheck] = {
    val delta = Math.abs(expectedValue - actualValue)
    reporter.log(f"Delta: $delta")
    if(delta > 0.001) throw new Exception("Analog Output Status tolerance not respected")
    reporter.log("Delta is within acceptable range (+/- 0.001)")
    None
  }

  override def verifyDelayMeasurementAccuracy(reporter: CheckReporter, delayMs: Long): Option[DeferredCheck] = {
    if (delayMs != 0) throw new Exception("Delay measurement was expected to be 0 ms")
    reporter.log("Delay measurement is within acceptable range (+/- 0)")
    None
  }

  override def verifyTimestamp(reporter: CheckReporter, points: Seq[GenIndexedPoint[MeasurementPoint]]): Option[DeferredCheck] = {
    verifyAllClassEvents(reporter, EventClass.All, points)
    None
  }

  override def verifyDeviceAttributes(reporter: CheckReporter, attributes: Seq[(Int, AttributeValue)]): Option[DeferredCheck] = {
    attributes.foreach { case (variation, value) =>
      trackingDatabase.getAttribute(variation) match {
        case Some(expected) if expected == value =>
        case Some(expected) => throw new Exception(s"g0v$variation is ${value.description}, expected ${expected.description}")
        case None => throw new Exception(s"g0v$variation is not defined in the outstation")
      }
    }
    reporter.log(s"Verified ${attributes.size} device attribute(s)")
    None
  }

  private def eventBufferConfig : EventBufferConfig = {
    val count = ushort(200)
    new EventBufferConfig(
      count,
      count,
      count,
      count,
      count,
      count,
      count,
      count
    )
  }

  private def startOutstation(): Unit = {
    // Create manager
    val runtimeConfig = new RuntimeConfig()
    runtimeConfig.numCoreThreads = ushort(1)
    this.runtime = new Runtime(runtimeConfig)

    // Create channel
    this.server = OutstationServer.createTcpServer(runtime, LinkErrorMode.DISCARD, s"${config.tcpConfig.address}:${config.tcpConfig.port}")

    // Create config
    val app = new CustomOutstationApplication
    app.coldRestartSupported = coldRestartSupported
    this.app = app
    val information = new CustomOutstationInformation
    this.controlHandler = new QueuedControlHandler(config.commandHandlerConfig.disableBinaryOutput, config.commandHandlerConfig.disableAnalogOutput)
    val listener = new CustomConnectionStateListener

    val dnp3Config = new io.stepfunc.dnp3.OutstationConfig(ushort(config.linkConfig.source), ushort(config.linkConfig.destination), eventBufferConfig)

    // Outstation config
    dnp3Config.features.selfAddress = config.linkConfig.selfAddressSupport
    dnp3Config.keepAliveTimeout = Duration.ofDays(30)
    dnp3Config.confirmTimeout = Duration.ofMillis(config.outstationConfig.confirmTimeoutMs)
    dnp3Config.selectTimeout = Duration.ofMillis(config.outstationConfig.selectTimeoutMs)
    dnp3Config.solicitedBufferSize = ushort(config.outstationConfig.fragmentSize)
    dnp3Config.unsolicitedBufferSize = ushort(config.outstationConfig.fragmentSize)
    dnp3Config.classZero.doubleBitBinary = false
    dnp3Config.classZero.octetString = false
    dnp3Config.maxControlsPerRequest = ushort(4)

    // Unsolicited responses config
    dnp3Config.features.unsolicited = config.unsolicitedResponseConfig.allowUnsolicited
    dnp3Config.features.broadcast = true
    dnp3Config.unsolicitedRetryDelay = Duration.ofMillis(config.unsolicitedResponseConfig.retryDelayMs)
    dnp3Config.maxUnsolicitedRetries = config.unsolicitedResponseConfig.maxNumRetries.map(x => uint(x)).getOrElse(UInteger.MAX)

    // Create the outstation
    this.outstation = server.addOutstation(dnp3Config, app, information, controlHandler, listener, AddressFilter.any())

    // Create the database
    this.trackingDatabase = new TrackingDatabase(app, this.outstation, config.testDatabaseConfig, this.controlHandler)
    app.onWriteStringAttribute = trackingDatabase.writeStringAttribute

    // Start the server
    this.server.bind()
  }

  private def shutdown(): Unit = {
    if(runtime != null) {
      runtime.shutdown()
      runtime = null
    }
  }
}
