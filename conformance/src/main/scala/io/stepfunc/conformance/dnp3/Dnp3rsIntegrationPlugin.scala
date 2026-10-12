package io.stepfunc.conformance.dnp3

import io.stepfunc.dnp4s.dnp3.app._
import io.stepfunc.dnp4s.dnp3.app.objects.AttributeValue
import io.stepfunc.dnp4s.dnp3.{ActionResult, ActionTiming, CheckReporter, DeferredCheck, EventExpectation, InputKind, IntegrationPlugin, PluginReporter}
import io.stepfunc.dnp4s.protocol.parsing.UInt8
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


  override def startProcedure(reporter: PluginReporter): Unit = {
    config = StackConfig.Default
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

  override def checkBinaryOutputOperate(reporter: CheckReporter, index: Int): Option[DeferredCheck] = {
    controlHandler.checkCrob(index)
    reporter.log(f"Binary output $index did operate")
    None
  }

  override def checkBinaryOutputNotOperate(reporter: CheckReporter): Option[DeferredCheck] = {
    controlHandler.checkNoCrob()
    reporter.log(f"Binary output did NOT operate")
    None
  }

  override def checkAnalogOutputOperate(reporter: CheckReporter, index: Int): Option[DeferredCheck] = {
    controlHandler.checkAnalog(index)
    reporter.log(f"Analog output $index did operate")
    None
  }

  override def checkAnalogOutputNotOperate(reporter: CheckReporter): Option[DeferredCheck] = {
    controlHandler.checkNoAnalog()
    reporter.log(f"Analog output did NOT operate")
    None
  }

  override def generateClassEvents(reporter: PluginReporter, eventClass: EventClass): Unit = {
    eventClass match {
      case EventClass.Class1 => generateInputChanges(reporter, InputKind.BinaryInput)
      case EventClass.Class2 => generateInputChanges(reporter, InputKind.Counter)
      case EventClass.Class3 => generateInputChanges(reporter, InputKind.AnalogInput)
      case _ => {
        generateInputChanges(reporter, InputKind.BinaryInput)
        this.eventBatch = this.eventBatch - 1
        generateInputChanges(reporter, InputKind.Counter)
        this.eventBatch = this.eventBatch - 1
        generateInputChanges(reporter, InputKind.AnalogInput)
      }
    }
  }

  override def generateExactClassEvents(reporter: PluginReporter, eventClass: EventClass, numEvents: Int): Unit = {
    eventClass match {
      case EventClass.Class1 => generateExactInputEvents(reporter, InputKind.BinaryInput, numEvents)
      case EventClass.Class2 => generateExactInputEvents(reporter, InputKind.Counter, numEvents)
      case EventClass.Class3 => generateExactInputEvents(reporter, InputKind.AnalogInput, numEvents)
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
    case InputKind.BinaryInput => printBinaryInput(reporter, trackingDatabase.generateBinaryInputEvent(idx, this.eventBatch))
    case InputKind.DoubleBitInput => printDoubleBitBinaryInput(reporter, trackingDatabase.generateDoubleBitBinaryInputEvent(idx, this.eventBatch))
    case InputKind.Counter => printCounter(reporter, trackingDatabase.generateCounterEvent(idx, this.eventBatch))
    case InputKind.AnalogInput => printAnalogInput(reporter, trackingDatabase.generateAnalogInputEvent(idx, this.eventBatch))
  }

  override def generateBinaryInputPattern(reporter: PluginReporter): Unit = {
    generateInputChanges(reporter, InputKind.BinaryInput)
  }

  override def generateDoubleBitBinaryInputPattern(reporter: PluginReporter): Unit = {
    generateInputChanges(reporter, InputKind.DoubleBitInput)
  }

  override def generateAnalogInputPattern(reporter: PluginReporter): Unit = {
    generateInputChanges(reporter, InputKind.AnalogInput)
  }

  override def generateTimeEvent(reporter: PluginReporter): Unit = {
    this.eventBatch = this.eventBatch + 1
    val event = trackingDatabase.generateBinaryInputEvent(0, this.eventBatch)
    printBinaryInput(reporter, event)
  }

  override def uninstallBinaryInputs(reporter: PluginReporter): Unit = {
    config = config.copy(testDatabaseConfig = config.testDatabaseConfig.copy(disableBinaryInputs = true))
    reporter.log("All BI points were uninstalled")
    cyclePower(reporter)
  }

  override def uninstallDoubleBitBinaryInputs(reporter: PluginReporter): Unit = {
    config = config.copy(testDatabaseConfig = config.testDatabaseConfig.copy(disableDoubleBitBinaryInputs = true))
    reporter.log("All DBBI points were uninstalled")
    cyclePower(reporter)
  }

  override def uninstallCounters(reporter: PluginReporter): Unit = {
    config = config.copy(testDatabaseConfig = config.testDatabaseConfig.copy(disableCounters = true))
    reporter.log("All counter points were uninstalled")
    cyclePower(reporter)
  }

  override def uninstallBinaryOutputs(reporter: PluginReporter): Unit = {
    config = config.copy(commandHandlerConfig = config.commandHandlerConfig.copy(disableBinaryOutput = true))
    reporter.log("All BO points were uninstalled")
    cyclePower(reporter)
  }

  override def uninstallAnalogOutputs(reporter: PluginReporter): Unit = {
    config = config.copy(commandHandlerConfig = config.commandHandlerConfig.copy(disableAnalogOutput = true))
    reporter.log("All AO points were uninstalled")
    cyclePower(reporter)
  }

  override def reinstallPoints(reporter: PluginReporter): Unit = {
    config = config.copy(
      testDatabaseConfig = config.testDatabaseConfig.copy(disableBinaryInputs = false, disableDoubleBitBinaryInputs = false, disableCounters = false),
      commandHandlerConfig = CommandHandlerConfig(disableBinaryOutput = false, disableAnalogOutput = false)
    )
    reporter.log("All points were reinstalled")
    cyclePower(reporter)
  }

  override def enableRemoteSupervisoryControl(reporter: PluginReporter): Unit = {
    config = config.copy(testDatabaseConfig =
      config.testDatabaseConfig.copy(isGlobalLocalControl = false, isSingleLocalControl = false))
    trackingDatabase.clearLocalControl()
    reporter.log("Every binary output is under remote control")
  }

  override def setGlobalRemoteSupervisoryControl(reporter: PluginReporter): Unit = {
    config = config.copy(testDatabaseConfig = config.testDatabaseConfig.copy(isGlobalLocalControl = true))
    trackingDatabase.setLocalControl(0 to 19)
    reporter.log("Global remote supervisory control was enabled")
  }

  override def setIndividualRemoteSupervisoryControl(reporter: PluginReporter, index: Int): Unit = {
    config = config.copy(testDatabaseConfig = config.testDatabaseConfig.copy(isSingleLocalControl = true))
    trackingDatabase.setLocalControl(Seq(0))
    reporter.log("Remote supervisory control was enabled")
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
    generateExactInputEvents(reporter, InputKind.BinaryInput, 1)
    ActionResult.Performed
  }

  override def setMaxFragmentSize(reporter: PluginReporter, maxFragmentSize: Int): Unit = {
    config = config.copy(outstationConfig = config.outstationConfig.copy(fragmentSize = maxFragmentSize))
    reporter.log(f"Max fragment size was set to $maxFragmentSize")
    cyclePower(reporter)
  }

  override def generateMultiFragmentResponse(reporter: PluginReporter): Unit = {
    generateExactInputEvents(reporter, InputKind.BinaryInput, 200)
    generateExactInputEvents(reporter, InputKind.DoubleBitInput, 200)
    generateExactInputEvents(reporter, InputKind.Counter, 200)
    generateExactInputEvents(reporter, InputKind.AnalogInput, 200)
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
    def getTypedEvent[T <: MeasurementPoint](event: GenIndexedPoint[MeasurementPoint])(implicit tag: ClassTag[T]): GenIndexedPoint[T] = {
      event.point match {
        case _: T => event.asInstanceOf[GenIndexedPoint[T]]
        case _ => throw new Exception("Received unexpected event type")
      }
    }
    expectedEvent match {
      case TypedEvent(_: BinaryInput) => checkBinaryInput(expectedEvent.asInstanceOf[TypedEvent[BinaryInput]], getTypedEvent(receivedEvent))
      case TypedEvent(_: DoubleBitBinaryInput) => checkDoubleBitBinaryInput(expectedEvent.asInstanceOf[TypedEvent[DoubleBitBinaryInput]], getTypedEvent(receivedEvent))
      case TypedEvent(_: Counter) => checkCounter(expectedEvent.asInstanceOf[TypedEvent[Counter]], getTypedEvent(receivedEvent))
      case TypedEvent(_: FrozenCounter) => checkFrozenCounter(expectedEvent.asInstanceOf[TypedEvent[FrozenCounter]], getTypedEvent(receivedEvent))
      case TypedEvent(_: AnalogInput) => checkAnalogInput(expectedEvent.asInstanceOf[TypedEvent[AnalogInput]], getTypedEvent(receivedEvent))
      case TypedEvent(_: BinaryOutputStatus) => checkBinaryOutput(expectedEvent.asInstanceOf[TypedEvent[BinaryOutputStatus]], getTypedEvent(receivedEvent))
      case TypedEvent(_: AnalogOutputStatus) => checkAnalogOutput(expectedEvent.asInstanceOf[TypedEvent[AnalogOutputStatus]], getTypedEvent(receivedEvent))
    }
  }

  private def verifyLatestClassEvents(reporter: CheckReporter, eventClass: EventClass, points: Seq[GenIndexedPoint[MeasurementPoint]]): Option[DeferredCheck] = {
    trackingDatabase.resetHandledEvents()

    def getTypedEvent[T <: MeasurementPoint](event: GenIndexedPoint[MeasurementPoint])(implicit tag: ClassTag[T]): GenIndexedPoint[T] = {
      event.point match {
        case _: T => event.asInstanceOf[GenIndexedPoint[T]]
        case _ => throw new Exception("Received unexpected event type")
      }
    }

    points.foreach(receivedEvent => {
      val expectedEvent = trackingDatabase.popEvent(BatchSpecifier.Specific(this.eventBatch), eventClass).getOrElse(throw new Exception("Unexpected event"))
      expectedEvent match {
        case TypedEvent(_: BinaryInput) => checkBinaryInput(expectedEvent.asInstanceOf[TypedEvent[BinaryInput]], getTypedEvent(receivedEvent))
        case TypedEvent(_: DoubleBitBinaryInput) => checkDoubleBitBinaryInput(expectedEvent.asInstanceOf[TypedEvent[DoubleBitBinaryInput]], getTypedEvent(receivedEvent))
        case TypedEvent(_: Counter) => checkCounter(expectedEvent.asInstanceOf[TypedEvent[Counter]], getTypedEvent(receivedEvent))
        case TypedEvent(_: FrozenCounter) => checkFrozenCounter(expectedEvent.asInstanceOf[TypedEvent[FrozenCounter]], getTypedEvent(receivedEvent))
        case TypedEvent(_: AnalogInput) => checkAnalogInput(expectedEvent.asInstanceOf[TypedEvent[AnalogInput]], getTypedEvent(receivedEvent))
        case TypedEvent(_: BinaryOutputStatus) => checkBinaryOutput(expectedEvent.asInstanceOf[TypedEvent[BinaryOutputStatus]], getTypedEvent(receivedEvent))
        case TypedEvent(_: AnalogOutputStatus) => checkAnalogOutput(expectedEvent.asInstanceOf[TypedEvent[AnalogOutputStatus]], getTypedEvent(receivedEvent))
      }
    })

    if (trackingDatabase.popEvent(BatchSpecifier.Specific(this.eventBatch), eventClass).isDefined) throw new Exception("Missing event")

    reporter.log(f"Verified ${points.size} $eventClass event(s)")
    None
  }

  override def verifyFirstClassEvent(reporter: CheckReporter, eventClass: EventClass, point: GenIndexedPoint[MeasurementPoint]): Option[DeferredCheck] = {
    def getTypedEvent[T <: MeasurementPoint](event: GenIndexedPoint[MeasurementPoint])(implicit tag: ClassTag[T]): GenIndexedPoint[T] = {
      event.point match {
        case _: T => event.asInstanceOf[GenIndexedPoint[T]]
        case _ => throw new Exception("Received unexpected event type")
      }
    }

    val expectedEvent = trackingDatabase.popEvent(BatchSpecifier.Specific(this.eventBatch), eventClass).getOrElse(throw new Exception("Unexpected event"))
    expectedEvent match {
      case TypedEvent(_: BinaryInput) => checkBinaryInput(expectedEvent.asInstanceOf[TypedEvent[BinaryInput]], getTypedEvent(point))
      case TypedEvent(_: DoubleBitBinaryInput) => checkDoubleBitBinaryInput(expectedEvent.asInstanceOf[TypedEvent[DoubleBitBinaryInput]], getTypedEvent(point))
      case TypedEvent(_: Counter) => checkCounter(expectedEvent.asInstanceOf[TypedEvent[Counter]], getTypedEvent(point))
      case TypedEvent(_: FrozenCounter) => checkFrozenCounter(expectedEvent.asInstanceOf[TypedEvent[FrozenCounter]], getTypedEvent(point))
      case TypedEvent(_: AnalogInput) => checkAnalogInput(expectedEvent.asInstanceOf[TypedEvent[AnalogInput]], getTypedEvent(point))
      case TypedEvent(_: BinaryOutputStatus) => checkBinaryOutput(expectedEvent.asInstanceOf[TypedEvent[BinaryOutputStatus]], getTypedEvent(point))
      case TypedEvent(_: AnalogOutputStatus) => checkAnalogOutput(expectedEvent.asInstanceOf[TypedEvent[AnalogOutputStatus]], getTypedEvent(point))
    }

    reporter.log(f"Verified first $eventClass event")
    None
  }

  private def verifyRestClassEvents(reporter: CheckReporter, eventClass: EventClass, points: Seq[GenIndexedPoint[MeasurementPoint]]): Option[DeferredCheck] = {
    def getTypedEvent[T <: MeasurementPoint](event: GenIndexedPoint[MeasurementPoint])(implicit tag: ClassTag[T]): GenIndexedPoint[T] = {
      event.point match {
        case _: T => event.asInstanceOf[GenIndexedPoint[T]]
        case _ => throw new Exception("Received unexpected event type")
      }
    }

    points.foreach(receivedEvent => {
      val expectedEvent = trackingDatabase.popEvent(BatchSpecifier.Specific(this.eventBatch), eventClass).getOrElse(throw new Exception("Unexpected event"))
      expectedEvent match {
        case TypedEvent(_: BinaryInput) => checkBinaryInput(expectedEvent.asInstanceOf[TypedEvent[BinaryInput]], getTypedEvent(receivedEvent))
        case TypedEvent(_: DoubleBitBinaryInput) => checkDoubleBitBinaryInput(expectedEvent.asInstanceOf[TypedEvent[DoubleBitBinaryInput]], getTypedEvent(receivedEvent))
        case TypedEvent(_: Counter) => checkCounter(expectedEvent.asInstanceOf[TypedEvent[Counter]], getTypedEvent(receivedEvent))
        case TypedEvent(_: FrozenCounter) => checkFrozenCounter(expectedEvent.asInstanceOf[TypedEvent[FrozenCounter]], getTypedEvent(receivedEvent))
        case TypedEvent(_: AnalogInput) => checkAnalogInput(expectedEvent.asInstanceOf[TypedEvent[AnalogInput]], getTypedEvent(receivedEvent))
        case TypedEvent(_: BinaryOutputStatus) => checkBinaryOutput(expectedEvent.asInstanceOf[TypedEvent[BinaryOutputStatus]], getTypedEvent(receivedEvent))
        case TypedEvent(_: AnalogOutputStatus) => checkAnalogOutput(expectedEvent.asInstanceOf[TypedEvent[AnalogOutputStatus]], getTypedEvent(receivedEvent))
      }
    })

    if (trackingDatabase.popEvent(BatchSpecifier.Specific(this.eventBatch), eventClass).isDefined) throw new Exception("Missing event")

    reporter.log(f"Verified ${points.size} $eventClass event(s)")
    None
  }

  private def verifyAllClassEvents(reporter: CheckReporter, eventClass: EventClass, points: Seq[GenIndexedPoint[MeasurementPoint]]): Option[DeferredCheck] = {
    trackingDatabase.resetHandledEvents()

    def getTypedEvent[T <: MeasurementPoint](event: GenIndexedPoint[MeasurementPoint])(implicit tag: ClassTag[T]): GenIndexedPoint[T] = {
      event.point match {
        case _: T => event.asInstanceOf[GenIndexedPoint[T]]
        case _ => throw new Exception("Received unexpected event type")
      }
    }

    points.foreach(receivedEvent => {
      val expectedEvent = trackingDatabase.popEvent(BatchSpecifier.All, eventClass).getOrElse(throw new Exception("Unexpected event"))
      expectedEvent match {
        case TypedEvent(_: BinaryInput) => checkBinaryInput(expectedEvent.asInstanceOf[TypedEvent[BinaryInput]], getTypedEvent(receivedEvent))
        case TypedEvent(_: DoubleBitBinaryInput) => checkDoubleBitBinaryInput(expectedEvent.asInstanceOf[TypedEvent[DoubleBitBinaryInput]], getTypedEvent(receivedEvent))
        case TypedEvent(_: Counter) => checkCounter(expectedEvent.asInstanceOf[TypedEvent[Counter]], getTypedEvent(receivedEvent))
        case TypedEvent(_: FrozenCounter) => checkFrozenCounter(expectedEvent.asInstanceOf[TypedEvent[FrozenCounter]], getTypedEvent(receivedEvent))
        case TypedEvent(_: AnalogInput) => checkAnalogInput(expectedEvent.asInstanceOf[TypedEvent[AnalogInput]], getTypedEvent(receivedEvent))
        case TypedEvent(_: BinaryOutputStatus) => checkBinaryOutput(expectedEvent.asInstanceOf[TypedEvent[BinaryOutputStatus]], getTypedEvent(receivedEvent))
        case TypedEvent(_: AnalogOutputStatus) => checkAnalogOutput(expectedEvent.asInstanceOf[TypedEvent[AnalogOutputStatus]], getTypedEvent(receivedEvent))
      }
    })

    if (trackingDatabase.popEvent(BatchSpecifier.All, eventClass).isDefined) throw new Exception("Missing event")

    reporter.log(f"Verified ${points.size} $eventClass event(s)")
    None
  }

  override def verifyInputCurrentValues[P <: MeasurementPoint](reporter: CheckReporter, kind: InputKind[P], points: Seq[GenIndexedPoint[P]]): Option[DeferredCheck] = {
    kind match {
      case InputKind.BinaryInput => verifyAllCurrentStatus(reporter, points, "binary input", trackingDatabase.getAllBinaryInputs, checkBinaryInput)
      case InputKind.DoubleBitInput => verifyAllCurrentStatus(reporter, points, "double-bit binary input", trackingDatabase.getAllDoubleBitBinaryInputs, checkDoubleBitBinaryInput)
      case InputKind.Counter => verifyAllCurrentStatus(reporter, points, "counter", trackingDatabase.getAllCounters, checkCounter)
      case InputKind.AnalogInput => verifyAllCurrentStatus(reporter, points, "analog input", trackingDatabase.getAllAnalogInputs, checkAnalogInput)
    }
    None
  }

  override def verifyRecordedCounterValues(reporter: CheckReporter, points: Seq[GenIndexedPoint[CounterPoint]]): Option[DeferredCheck] = {
    verifyAllCurrentStatus(reporter, points, "counter", trackingDatabase.getRecordedCounters, checkCounter)
    None
  }

  override def verifyInputEvents[P <: MeasurementPoint](reporter: CheckReporter, kind: InputKind[P], expectation: EventExpectation, points: Seq[GenIndexedPoint[P]]): Option[DeferredCheck] = {
    kind match {
      case InputKind.BinaryInput => verifyEvents(reporter, expectation, points, "binary input", trackingDatabase.popBinaryEvent, checkBinaryInput)
      case InputKind.DoubleBitInput => verifyEvents(reporter, expectation, points, "double-bit binary input", trackingDatabase.popDoubleBitBinaryEvent, checkDoubleBitBinaryInput)
      case InputKind.Counter if expectation == EventExpectation.CurrentValuesWithOptionalEarlier =>
        verifyCurrentValues(reporter, points, "counter", trackingDatabase.getAllCounterEvents(BatchSpecifier.All), checkCounter)
      case InputKind.AnalogInput if expectation == EventExpectation.CurrentValuesWithOptionalEarlier =>
        verifyCurrentValues(reporter, points, "analog input", trackingDatabase.getAllAnalogInputEvents(BatchSpecifier.All), checkAnalogInput)
      case InputKind.Counter => verifyEvents(reporter, expectation, points, "counter", trackingDatabase.popCounterEvent, checkCounter)
      case InputKind.AnalogInput => verifyEvents(reporter, expectation, points, "analog input", trackingDatabase.popAnalogInputEvent, checkAnalogInput)
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
      case InputKind.BinaryInput => verifyFirstEvent(reporter, point, "binary input", trackingDatabase.popBinaryEvent, checkBinaryInput)
      case InputKind.DoubleBitInput => verifyFirstEvent(reporter, point, "double-bit binary input", trackingDatabase.popDoubleBitBinaryEvent, checkDoubleBitBinaryInput)
      case InputKind.Counter => verifyFirstEvent(reporter, point, "counter", trackingDatabase.popCounterEvent, checkCounter)
      case InputKind.AnalogInput => verifyFirstEvent(reporter, point, "analog input", trackingDatabase.popAnalogInputEvent, checkAnalogInput)
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
    if (polledResponsePoints.nonEmpty) throw new Exception("Expected no events in polled response")
    None
  }

  override def verifyResponseRetainsEvents(reporter: CheckReporter, previousResponsePoints: Seq[GenIndexedPoint[MeasurementPoint]], currentResponsePoints: Seq[GenIndexedPoint[MeasurementPoint]]): Option[DeferredCheck] = {
    if (!previousResponsePoints.equals(currentResponsePoints)) {
      throw new Exception("Received points are different")
    }
    reporter.log("Verified that received points are the same as the previously received points")
    None
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
    if ((expectedValue.value.value == DoubleBit.DETERMINED_OFF && receivedValue.point.value != io.stepfunc.dnp4s.dnp3.app.DoubleBit.DeterminedOff) ||
      (expectedValue.value.value == DoubleBit.DETERMINED_ON && receivedValue.point.value != io.stepfunc.dnp4s.dnp3.app.DoubleBit.DeterminedOn) ||
      (expectedValue.value.value == DoubleBit.INDETERMINATE && receivedValue.point.value != io.stepfunc.dnp4s.dnp3.app.DoubleBit.Indeterminate) ||
      (expectedValue.value.value == DoubleBit.INTERMEDIATE && receivedValue.point.value != io.stepfunc.dnp4s.dnp3.app.DoubleBit.Intermediate)) {
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
    this.trackingDatabase = new TrackingDatabase(app, this.outstation, config.testDatabaseConfig)
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
