// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package io.stepfunc.conformance.dnp3

import io.stepfunc.dnp3c.dnp3.PointKind
import io.stepfunc.dnp3c.dnp3.app.EventClass
import io.stepfunc.dnp3c.dnp3.app.objects.AttributeValue
import io.stepfunc.dnp3._
import org.joou.UByte
import org.joou.Unsigned.{ubyte, uint, ushort}

import java.util.concurrent.ConcurrentHashMap
import scala.collection.mutable
import scala.collection.mutable.ArrayBuffer

sealed trait BatchSpecifier
object BatchSpecifier {
  case object All extends BatchSpecifier
  case class Specific(batchId: Int) extends BatchSpecifier
}

trait Event {
  def idx: Int
  def batch: Int
  def eventClass: EventClass
  def isHandled: Boolean
  def handle(): Unit
  def reset(): Unit
}

object TypedEvent {
  def unapply[T](x: TypedEvent[T]): Option[Any] = Some(x.value)
}
class TypedEvent[T](val idx: Int, val batch: Int, val eventClass: EventClass, val value: T) extends Event {
  var isHandled: Boolean = false

  override def handle(): Unit = {
    this.isHandled = true
  }

  override def reset(): Unit = {
    this.isHandled = false
  }
}

class TrackingDatabase(val app: CustomOutstationApplication, val outstation: Outstation, testDatabaseConfig: TestDatabaseConfig, controlHandler: QueuedControlHandler) {
  // Create all the tracking points
  // inputs 0 to 9 are the ones class 1 events change; 10 and 11 start in class 2 for 3.1r1's 8.26.2
  // (device.ini's class.assign.class2BinaryInputs)
  private val binaryPoints = mutable.SortedMap((for (i <- 0 to 11) yield i -> new BinaryInput(ushort(i), false, new Flags(ubyte(0x01)), Timestamp.invalidTimestamp())): _*)
  // the class of each binary input's events, which ASSIGN_CLASS changes; None for no events
  private val binaryClasses = mutable.Map((for (i <- 0 to 11) yield i -> Option(if (i >= 10) EventClass.Class2 else EventClass.Class1)): _*)
  private val doubleBitPoints = mutable.SortedMap((for (i <- 0 to 9) yield i -> new DoubleBitBinaryInput(ushort(i), DoubleBit.DETERMINED_OFF, new Flags(ubyte(0x41)), Timestamp.invalidTimestamp())): _*)
  private val counters = mutable.SortedMap((for (i <- 0 to 9) yield i -> new Counter(ushort(i), uint(0), new Flags(ubyte(0x01)), Timestamp.invalidTimestamp())): _*)
  private val analogInputs = mutable.SortedMap((for (i <- 0 to 9) yield i -> new AnalogInput(ushort(i), 0.0, new Flags(ubyte(0x01)), Timestamp.invalidTimestamp())): _*)
  // the frozen counters as the database starts them, changed only by generateFrozenCounterEvent and setFlags
  private val frozenCounters = mutable.SortedMap((for (i <- 0 to 9) yield i -> new FrozenCounter(ushort(i), uint(0), new Flags(ubyte(0x01)), Timestamp.invalidTimestamp())): _*)

  private val binaryOutputs = mutable.SortedMap((for (i <- 0 to 19) yield i -> new BinaryOutputStatus(ushort(i), false, new Flags(ubyte(0x01)), Timestamp.invalidTimestamp())): _*)
  private val analogOutputs = mutable.SortedMap((for (i <- 0 to 19) yield i -> new AnalogOutputStatus(ushort(i), 0.0, new Flags(ubyte(0x01)), Timestamp.invalidTimestamp())): _*)

  private val events: ArrayBuffer[Event] = ArrayBuffer()
  private var recordedCounterValues: mutable.SortedMap[Int, Counter] = mutable.SortedMap()

  // standard set device attributes as the master should read them, updated when it writes the writable ones
  private val attributes = new ConcurrentHashMap[Int, AttributeValue]()

  // Initialize the outstation database
  outstation.transaction(db => {
    val updateOptions = UpdateOptions.noEvent()

    def checkDefined(variation: UByte, error: AttrDefError): Unit =
      if (error != AttrDefError.OK) throw new Exception(s"Unable to define attribute g0v$variation: $error")

    // the writable attributes are user-assigned names
    List(
      (AttributeVariations.DEVICE_MANUFACTURERS_NAME, false, "Step Function I/O"),
      (AttributeVariations.PRODUCT_NAME_AND_MODEL, false, "dnp3 conformance outstation"),
      (AttributeVariations.USER_ASSIGNED_LOCATION, true, "Bend, OR"),
      (AttributeVariations.USER_ASSIGNED_DEVICE_NAME, true, "DUT")
    ).foreach { case (variation, writable, value) =>
      checkDefined(variation, db.defineStringAttr(ubyte(0), writable, variation, value))
      attributes.put(variation.intValue, AttributeValue.VisibleString(value))
    }
    checkDefined(
      AttributeVariations.MAX_RX_FRAGMENT_SIZE,
      db.defineUintAttr(ubyte(0), false, AttributeVariations.MAX_RX_FRAGMENT_SIZE, uint(2048)))
    attributes.put(AttributeVariations.MAX_RX_FRAGMENT_SIZE.intValue, AttributeValue.UnsignedInt(2048))

    if(!testDatabaseConfig.disableBinaryInputs) {
      val binaryConfig = new BinaryInputConfig()
      // with time, so the event generateTimeEvent makes carries its time (8.7.2.2)
      binaryConfig.eventVariation = EventBinaryInputVariation.GROUP2_VAR2

      this.binaryPoints.values.foreach(e => {
        db.addBinaryInput(e.index, TrackingDatabase.toFfi(binaryClasses(e.index.intValue)), binaryConfig)
        db.updateBinaryInput(e, updateOptions)
      })
    }

    if(!testDatabaseConfig.disableDoubleBitBinaryInputs) {
      val doubleBitBinaryConfig = new DoubleBitBinaryInputConfig()

      this.doubleBitPoints.values.foreach(e => {
        db.addDoubleBitBinaryInput(e.index, io.stepfunc.dnp3.EventClass.CLASS1, doubleBitBinaryConfig)
        db.updateDoubleBitBinaryInput(e, updateOptions)
      })
    }

    if(!testDatabaseConfig.disableCounters) {
      val counterConfig = new CounterConfig()
      counterConfig.staticVariation = StaticCounterVariation.GROUP20_VAR2

      val frozenCounterConfig = new FrozenCounterConfig()
      frozenCounterConfig.staticVariation = StaticFrozenCounterVariation.GROUP21_VAR2

      this.counters.values.foreach(e => {
        db.addCounter(e.index, io.stepfunc.dnp3.EventClass.CLASS2, counterConfig)
        db.updateCounter(e, updateOptions)

        if (!testDatabaseConfig.disableFrozenCounters) {
          val frozenCounter = new FrozenCounter(e.index, e.value, e.flags, e.time)
          db.addFrozenCounter(e.index, io.stepfunc.dnp3.EventClass.CLASS1, frozenCounterConfig)
          db.updateFrozenCounter(frozenCounter, updateOptions)
        }
      })
    }

    if (!testDatabaseConfig.disableAnalogInputs) this.analogInputs.values.foreach(e => {
      val aiConfig = new AnalogInputConfig()
      aiConfig.staticVariation = StaticAnalogInputVariation.GROUP30_VAR2

      db.addAnalogInput(e.index, io.stepfunc.dnp3.EventClass.CLASS3, aiConfig)
      db.updateAnalogInput(e, updateOptions)
    })

    if (!testDatabaseConfig.disableBinaryOutputStatus) this.binaryOutputs.values.foreach(e => {
      val boConfig = new BinaryOutputStatusConfig()
      boConfig.staticVariation = StaticBinaryOutputStatusVariation.GROUP10_VAR2

      db.addBinaryOutputStatus(e.index, io.stepfunc.dnp3.EventClass.NONE, boConfig)
      db.updateBinaryOutputStatus(e, updateOptions)
    })

    if (!testDatabaseConfig.disableAnalogOutputStatus) this.analogOutputs.values.foreach(e => {
      val aoConfig = new AnalogOutputStatusConfig()
      aoConfig.staticVariation = StaticAnalogOutputStatusVariation.GROUP40_VAR2

      db.addAnalogOutputStatus(e.index, io.stepfunc.dnp3.EventClass.CLASS1, aoConfig)
      db.updateAnalogOutputStatus(e, updateOptions)
    })
  })

  if(testDatabaseConfig.isGlobalLocalControl) setLocalControl(binaryOutputs.keys)
  testDatabaseConfig.localControlIndex.foreach(idx => setLocalControl(Seq(idx)))

  // Places binary outputs in the local state, like flipping a switch on the device
  def setLocalControl(indices: Iterable[Int]): Unit = {
    app.isLocalControl = true
    controlHandler.localOutputs = controlHandler.localOutputs ++ indices
    outstation.transaction(db => {
      indices.foreach(idx => {
        val point = binaryOutputs(idx)
        // ONLINE cleared, the state and every other flag kept: LOCAL_FORCED means a value forced locally, not local mode
        // (IEEE 1815-2012 Table 11-5; 3.1r1's 8.6.4 step 1.9)
        val newPoint = new BinaryOutputStatus(point.index, point.value, new Flags(ubyte(point.flags.value.intValue() & ~0x01)), point.time)
        binaryOutputs(idx) = newPoint
        db.updateBinaryOutputStatus(newPoint, UpdateOptions.noEvent())
      })
    })
  }

  // Puts every binary output back under remote control
  def clearLocalControl(): Unit = {
    app.isLocalControl = false
    controlHandler.localOutputs = Set.empty
    outstation.transaction(db => {
      binaryOutputs.keys.toList.foreach(idx => {
        val point = binaryOutputs(idx)
        val newPoint = new BinaryOutputStatus(point.index, point.value, new Flags(ubyte(point.flags.value.intValue() | 0x01)), point.time)
        binaryOutputs(idx) = newPoint
        db.updateBinaryOutputStatus(newPoint, UpdateOptions.noEvent())
      })
    })
  }

  def getAttribute(variation: Int): Option[AttributeValue] = Option(attributes.get(variation))

  // a point header of an accepted ASSIGN_CLASS request: later events of its points take the class.
  // 8.26.2 assigns only the binary inputs device.ini names, so other types aren't tracked
  def assignClass(pointType: PointType, range: Option[(Int, Int)], pointClass: io.stepfunc.dnp3.EventClass): Unit =
    if (pointType == PointType.BINARY_INPUT) this.synchronized {
      val indices = binaryClasses.keys.filter(i => range.forall { case (start, stop) => i >= start && i <= stop })
      indices.foreach(i => binaryClasses(i) = TrackingDatabase.fromFfi(pointClass))
    }

  def writeStringAttribute(variation: Int, value: String): Unit =
    attributes.put(variation, AttributeValue.VisibleString(value))

  def generateBinaryInputEvent(idx: Int, eventBatch: Int): TypedEvent[BinaryInput] = {
    val value = !binaryPoints(idx).value
    val flags = if(value) ubyte(0x81) else ubyte(0x01)
    val newPoint = new BinaryInput(ushort(idx), value, new Flags(flags), app.now())

    this.binaryPoints(idx) = newPoint
    // a point assigned to no class changes without an event, which is logged but not expected
    val eventClass = this.synchronized(binaryClasses(idx))
    val newEvent = new TypedEvent[BinaryInput](idx, eventBatch, eventClass.getOrElse(EventClass.All), newPoint)
    eventClass.foreach(_ => this.events.append(newEvent))

    outstation.transaction(db => {
      db.updateBinaryInput(newPoint, UpdateOptions.detectEvent())
    })

    newEvent
  }

  def generateDoubleBitBinaryInputEvent(idx: Int, eventBatch: Int): TypedEvent[DoubleBitBinaryInput] = {
    val value = if (doubleBitPoints(idx).value == DoubleBit.DETERMINED_OFF) DoubleBit.DETERMINED_ON else DoubleBit.DETERMINED_OFF
    val flags = if (value == DoubleBit.DETERMINED_OFF) ubyte(0x41) else ubyte(0x81)
    val newPoint = new DoubleBitBinaryInput(ushort(idx), value, new Flags(flags), app.now())

    this.doubleBitPoints(idx) = newPoint
    val newEvent = new TypedEvent[DoubleBitBinaryInput](idx, eventBatch, EventClass.All, newPoint)
    this.events.append(newEvent)

    outstation.transaction(db => {
      db.updateDoubleBitBinaryInput(newPoint, UpdateOptions.detectEvent())
    })

    newEvent
  }

  def generateCounterEvent(idx: Int, eventBatch: Int): TypedEvent[Counter] = {
    val value = counters(idx).value.longValue() + 1
    val newPoint = new Counter(ushort(idx), uint(value), new Flags(ubyte(0x01)), app.now())

    this.counters(idx) = newPoint
    val newEvent = new TypedEvent[Counter](idx, eventBatch, EventClass.Class2, newPoint)
    this.events.append(newEvent)

    outstation.transaction(db => {
      db.updateCounter(newPoint, UpdateOptions.detectEvent())
    })

    newEvent
  }

  def recordCurrentCounterValues(): Unit = {
    this.recordedCounterValues = this.counters.clone()
  }

  def generateAnalogInputEvent(idx: Int, eventBatch: Int): TypedEvent[AnalogInput] = {
    val value = analogInputs(idx).value + 100.0
    val newPoint = new AnalogInput(ushort(idx), value, new Flags(ubyte(0x01)), app.now())

    this.analogInputs(idx) = newPoint
    val newEvent = new TypedEvent[AnalogInput](idx, eventBatch, EventClass.Class3, newPoint)
    this.events.append(newEvent)

    outstation.transaction(db => {
      db.updateAnalogInput(newPoint, UpdateOptions.detectEvent())
    })

    newEvent
  }

  // a frozen counter event: the frozen value goes up by one (3.1r1's 8.25)
  def generateFrozenCounterEvent(idx: Int, eventBatch: Int): TypedEvent[FrozenCounter] = {
    val value = frozenCounters(idx).value.longValue() + 1
    val newPoint = new FrozenCounter(ushort(idx), uint(value), new Flags(ubyte(0x01)), app.now())

    this.frozenCounters(idx) = newPoint
    val newEvent = new TypedEvent[FrozenCounter](idx, eventBatch, EventClass.Class1, newPoint)
    this.events.append(newEvent)

    outstation.transaction(db => {
      db.updateFrozenCounter(newPoint, UpdateOptions.detectEvent())
    })

    newEvent
  }

  // an analog input set to a value, with the event that makes (3.1r1's 8.19.2.4)
  def setAnalogInputValue(idx: Int, value: Double, eventBatch: Int): TypedEvent[AnalogInput] = {
    val newPoint = new AnalogInput(ushort(idx), value, new Flags(ubyte(0x01)), app.now())

    this.analogInputs(idx) = newPoint
    val newEvent = new TypedEvent[AnalogInput](idx, eventBatch, EventClass.Class3, newPoint)
    this.events.append(newEvent)

    outstation.transaction(db => {
      db.updateAnalogInput(newPoint, UpdateOptions.detectEvent())
    })

    newEvent
  }

  // Sets the flag bits of the points of a kind, all of them or those given, keeping each point's state and value, without
  // events (3.1r1's 8.24 and 8.25). The state bits of binary points are kept in their flags octet.
  def setFlags(kind: PointKind, indices: Option[Seq[Int]], bits: Int): Unit = {
    def state(on: Boolean): Int = if (on) 0x80 else 0x00
    def doubleBitState(value: DoubleBit): Int = value match {
      case DoubleBit.INTERMEDIATE => 0x00
      case DoubleBit.DETERMINED_OFF => 0x40
      case DoubleBit.DETERMINED_ON => 0x80
      case _ => 0xC0
    }
    def flags(b: Int): Flags = new Flags(ubyte(b))
    def which[T](points: mutable.SortedMap[Int, T]): Seq[Int] = indices.getOrElse(points.keys.toSeq)
    outstation.transaction(db => {
      kind match {
        case PointKind.BinaryInput => which(binaryPoints).foreach { i =>
          val p = binaryPoints(i)
          binaryPoints(i) = new BinaryInput(p.index, p.value, flags(bits | state(p.value)), p.time)
          db.updateBinaryInput(binaryPoints(i), UpdateOptions.noEvent())
        }
        case PointKind.DoubleBitInput => which(doubleBitPoints).foreach { i =>
          val p = doubleBitPoints(i)
          doubleBitPoints(i) = new DoubleBitBinaryInput(p.index, p.value, flags(bits | doubleBitState(p.value)), p.time)
          db.updateDoubleBitBinaryInput(doubleBitPoints(i), UpdateOptions.noEvent())
        }
        case PointKind.BinaryOutputStatus => which(binaryOutputs).foreach { i =>
          val p = binaryOutputs(i)
          binaryOutputs(i) = new BinaryOutputStatus(p.index, p.value, flags(bits | state(p.value)), p.time)
          db.updateBinaryOutputStatus(binaryOutputs(i), UpdateOptions.noEvent())
        }
        case PointKind.Counter => which(counters).foreach { i =>
          val p = counters(i)
          counters(i) = new Counter(p.index, p.value, flags(bits), p.time)
          db.updateCounter(counters(i), UpdateOptions.noEvent())
        }
        case PointKind.FrozenCounter => which(frozenCounters).foreach { i =>
          val p = frozenCounters(i)
          frozenCounters(i) = new FrozenCounter(p.index, p.value, flags(bits), p.time)
          db.updateFrozenCounter(frozenCounters(i), UpdateOptions.noEvent())
        }
        case PointKind.AnalogInput => which(analogInputs).foreach { i =>
          val p = analogInputs(i)
          analogInputs(i) = new AnalogInput(p.index, p.value, flags(bits), p.time)
          db.updateAnalogInput(analogInputs(i), UpdateOptions.noEvent())
        }
        case PointKind.AnalogOutputStatus => which(analogOutputs).foreach { i =>
          val p = analogOutputs(i)
          analogOutputs(i) = new AnalogOutputStatus(p.index, p.value, flags(bits), p.time)
          db.updateAnalogOutputStatus(analogOutputs(i), UpdateOptions.noEvent())
        }
      }
    })
  }

  def getAllFrozenCounterEvents(batch: BatchSpecifier): Seq[TypedEvent[FrozenCounter]] = {
    getAllEvents(batch).flatMap {
      case event @ TypedEvent(_: FrozenCounter) => Some(event.asInstanceOf[TypedEvent[FrozenCounter]])
      case _ => None
    }
  }

  def popFrozenCounterEvent(batch: BatchSpecifier): Option[TypedEvent[FrozenCounter]] = {
    val head = getAllFrozenCounterEvents(batch).find(e => !e.isHandled)
    head.foreach(_.handle())
    head
  }

  def resetHandledEvents(): Unit = {
    this.events.foreach(e => e.reset())
  }

  def getAllPoints(isClass0: Boolean): Set[Event] = {
    (
      getAllBinaryInputs.map(e => e.asInstanceOf[Event]).toList :::
      (if (!isClass0) getAllDoubleBitBinaryInputs.map(_.asInstanceOf[Event]).toList else Nil) :::
      getAllCounters.map(_.asInstanceOf[Event]).toList :::
      this.frozenCounters.map(e => new TypedEvent[FrozenCounter](e._1, 0, EventClass.All, e._2).asInstanceOf[Event]).toList :::
      getAllAnalogInputs.map(_.asInstanceOf[Event]).toList :::
      this.binaryOutputs.map(e => new TypedEvent[BinaryOutputStatus](e._1, 0, EventClass.All, e._2).asInstanceOf[Event]).toList :::
      this.analogOutputs.map(e => new TypedEvent[AnalogOutputStatus](e._1, 0, EventClass.All, e._2).asInstanceOf[Event]).toList
    ).toSet
  }

  def getAllEvents(batch: BatchSpecifier, eventClass: EventClass = EventClass.All): Seq[Event] = {
    this.events.filter(e => {
      batch match {
        case BatchSpecifier.All => true
        case BatchSpecifier.Specific(id) => e.batch == id
      }
    }).filter(e => {
      eventClass match {
        case EventClass.All => true
        case _ => e.eventClass == eventClass
      }
    }).toSeq
  }

  def getUnhandledEvents(batch: BatchSpecifier, eventClass: EventClass): Seq[Event] = {
    getAllEvents(batch, eventClass).filter(e => !e.isHandled)
  }

  def popEvent(batch: BatchSpecifier, eventClass: EventClass): Option[Event] = {
    val head = getUnhandledEvents(batch, eventClass).headOption
    head match {
      case Some(event) => event.handle()
      case None =>
    }
    head
  }

  def getAllBinaryInputs: Seq[TypedEvent[BinaryInput]] = {
    this.binaryPoints.map(e => {
      new TypedEvent(e._1, 0, EventClass.All, e._2)
    }).toSeq
  }

  def getAllBinaryInputEvents(batch: BatchSpecifier): Seq[TypedEvent[BinaryInput]] = {
    getAllEvents(batch).flatMap {
      case event @ TypedEvent(_: BinaryInput) => Some(event.asInstanceOf[TypedEvent[BinaryInput]])
      case _ => None
    }
  }

  def getUnhandledBinaryInputEvents(batch: BatchSpecifier): Seq[TypedEvent[BinaryInput]] = {
    getAllBinaryInputEvents(batch).filter(e => !e.isHandled)
  }

  def popBinaryEvent(batch: BatchSpecifier): Option[TypedEvent[BinaryInput]] = {
    val head = getUnhandledBinaryInputEvents(batch).headOption
    head match {
      case Some(event) => event.handle()
      case None =>
    }
    head
  }

  def getAllDoubleBitBinaryInputs: Seq[TypedEvent[DoubleBitBinaryInput]] = {
    this.doubleBitPoints.map(e => {
      new TypedEvent(e._1, 0, EventClass.All, e._2)
    }).toSeq
  }

  def getAllDoubleBitBinaryInputEvents(batch: BatchSpecifier): Seq[TypedEvent[DoubleBitBinaryInput]] = {
    getAllEvents(batch).flatMap {
      case event @ TypedEvent(_: DoubleBitBinaryInput) => Some(event.asInstanceOf[TypedEvent[DoubleBitBinaryInput]])
      case _ => None
    }
  }

  def getUnhandledDoubleBitBinaryInputEvents(batch: BatchSpecifier): Seq[TypedEvent[DoubleBitBinaryInput]] = {
    getAllDoubleBitBinaryInputEvents(batch).filter(e => !e.isHandled)
  }

  def popDoubleBitBinaryEvent(batch: BatchSpecifier): Option[TypedEvent[DoubleBitBinaryInput]] = {
    val head = getUnhandledDoubleBitBinaryInputEvents(batch).headOption
    head match {
      case Some(event) => event.handle()
      case None =>
    }
    head
  }

  def getAllCounters: Seq[TypedEvent[Counter]] = {
    this.counters.map(e => {
      new TypedEvent(e._1, 0, EventClass.All, e._2)
    }).toSeq
  }

  def getRecordedCounters: Seq[TypedEvent[Counter]] = {
    this.recordedCounterValues.map(e => {
      new TypedEvent(e._1, 0, EventClass.All, e._2)
    }).toSeq
  }

  def getAllCounterEvents(batch: BatchSpecifier): Seq[TypedEvent[Counter]] = {
    getAllEvents(batch).flatMap {
      case event @ TypedEvent(_: Counter) => Some(event.asInstanceOf[TypedEvent[Counter]])
      case _ => None
    }
  }

  def getUnhandledCounterEvents(batch: BatchSpecifier): Seq[TypedEvent[Counter]] = {
    getAllCounterEvents(batch).filter(e => !e.isHandled)
  }

  def popCounterEvent(batch: BatchSpecifier): Option[TypedEvent[Counter]] = {
    val head = getUnhandledCounterEvents(batch).headOption
    head match {
      case Some(event) => event.handle()
      case None =>
    }
    head
  }

  def getAllAnalogInputs: Seq[TypedEvent[AnalogInput]] = {
    this.analogInputs.map(e => {
      new TypedEvent(e._1, 0, EventClass.All, e._2)
    }).toSeq
  }

  def getAllAnalogInputEvents(batch: BatchSpecifier): Seq[TypedEvent[AnalogInput]] = {
    getAllEvents(batch).flatMap {
      case event @ TypedEvent(_: AnalogInput) => Some(event.asInstanceOf[TypedEvent[AnalogInput]])
      case _ => None
    }
  }

  def getUnhandledAnalogInputEvents(batch: BatchSpecifier): Seq[TypedEvent[AnalogInput]] = {
    getAllAnalogInputEvents(batch).filter(e => !e.isHandled)
  }

  def popAnalogInputEvent(batch: BatchSpecifier): Option[TypedEvent[AnalogInput]] = {
    val head = getUnhandledAnalogInputEvents(batch).headOption
    head match {
      case Some(event) => event.handle()
      case None =>
    }
    head
  }
}

object TrackingDatabase {
  def toFfi(eventClass: Option[EventClass]): io.stepfunc.dnp3.EventClass = eventClass match {
    case Some(EventClass.Class1) => io.stepfunc.dnp3.EventClass.CLASS1
    case Some(EventClass.Class2) => io.stepfunc.dnp3.EventClass.CLASS2
    case Some(EventClass.Class3) => io.stepfunc.dnp3.EventClass.CLASS3
    case _ => io.stepfunc.dnp3.EventClass.NONE
  }

  def fromFfi(eventClass: io.stepfunc.dnp3.EventClass): Option[EventClass] = eventClass match {
    case io.stepfunc.dnp3.EventClass.CLASS1 => Some(EventClass.Class1)
    case io.stepfunc.dnp3.EventClass.CLASS2 => Some(EventClass.Class2)
    case io.stepfunc.dnp3.EventClass.CLASS3 => Some(EventClass.Class3)
    case _ => None
  }
}
