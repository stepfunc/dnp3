package io.stepfunc.dnp3_conformance

import com.automatak.dnp4s.dnp3.app.EventClass
import io.stepfunc.dnp3._
import org.joou.Unsigned.{ubyte, uint, ushort}

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

class TrackingDatabase(val app: CustomOutstationApplication, val outstation: Outstation, testDatabaseConfig: TestDatabaseConfig) {
  // Create all the tracking points
  private val binaryPoints = mutable.SortedMap((for (i <- 0 to 9) yield i -> new Binary(ushort(i), false, new Flags(ubyte(0x01)), Timestamp.invalidTimestamp())): _*)
  private val doubleBitPoints = mutable.SortedMap((for (i <- 0 to 9) yield i -> new DoubleBitBinary(ushort(i), DoubleBit.DETERMINED_OFF, new Flags(ubyte(0x41)), Timestamp.invalidTimestamp())): _*)
  private val counters = mutable.SortedMap((for (i <- 0 to 9) yield i -> new Counter(ushort(i), uint(0), new Flags(ubyte(0x01)), Timestamp.invalidTimestamp())): _*)
  private val analogInputs = mutable.SortedMap((for (i <- 0 to 9) yield i -> new Analog(ushort(i), 0.0, new Flags(ubyte(0x01)), Timestamp.invalidTimestamp())): _*)

  private val binaryOutputs = mutable.SortedMap((for (i <- 0 to 19) yield i -> new BinaryOutputStatus(ushort(i), false, new Flags(ubyte(0x01)), Timestamp.invalidTimestamp())): _*)
  private val analogOutputs = mutable.SortedMap((for (i <- 0 to 19) yield i -> new AnalogOutputStatus(ushort(i), 0.0, new Flags(ubyte(0x01)), Timestamp.invalidTimestamp())): _*)

  private val events: ArrayBuffer[Event] = ArrayBuffer()
  private var recordedCounterValues: mutable.SortedMap[Int, Counter] = mutable.SortedMap()

  if(testDatabaseConfig.isGlobalLocalControl) {
    this.binaryOutputs.foreach(originalPoint => {
      this.binaryOutputs(originalPoint._1) = new BinaryOutputStatus(originalPoint._2.index, originalPoint._2.value, new Flags(ubyte(0x11)), originalPoint._2.time)
    })
  }

  if(testDatabaseConfig.isSingleLocalControl) {
    val originalPoint = this.binaryOutputs(0)
    val newPoint = new BinaryOutputStatus(originalPoint.index, originalPoint.value, new Flags(ubyte(0x11)), originalPoint.time)
    this.binaryOutputs(0) = newPoint
  }

  // Initialize the outstation database
  outstation.transaction(db => {
    val updateOptions = new UpdateOptions()
    updateOptions.eventMode = EventMode.SUPPRESS

    if(!testDatabaseConfig.disableBinaryInputs) {
      val binaryConfig = new BinaryConfig()

      this.binaryPoints.values.foreach(e => {
        db.addBinary(e.index, io.stepfunc.dnp3.EventClass.CLASS1, binaryConfig)
        db.updateBinary(e, updateOptions)
      })
    }

    if(!testDatabaseConfig.disableDoubleBitBinaryInputs) {
      val doubleBitBinaryConfig = new DoubleBitBinaryConfig()

      this.doubleBitPoints.values.foreach(e => {
        db.addDoubleBitBinary(e.index, io.stepfunc.dnp3.EventClass.CLASS1, doubleBitBinaryConfig)
        db.updateDoubleBitBinary(e, updateOptions)
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

        val frozenCounter = new FrozenCounter(e.index, e.value, e.flags, e.time)
        db.addFrozenCounter(e.index, io.stepfunc.dnp3.EventClass.CLASS1, frozenCounterConfig)
        db.updateFrozenCounter(frozenCounter, updateOptions)
      })
    }

    this.analogInputs.values.foreach(e => {
      val aiConfig = new AnalogConfig()
      aiConfig.staticVariation = StaticAnalogVariation.GROUP30_VAR2

      db.addAnalog(e.index, io.stepfunc.dnp3.EventClass.CLASS3, aiConfig)
      db.updateAnalog(e, updateOptions)
    })

    this.binaryOutputs.values.foreach(e => {
      val boConfig = new BinaryOutputStatusConfig()
      boConfig.staticVariation = StaticBinaryOutputStatusVariation.GROUP10_VAR2

      db.addBinaryOutputStatus(e.index, io.stepfunc.dnp3.EventClass.NONE, boConfig)
      db.updateBinaryOutputStatus(e, updateOptions)
    })

    this.analogOutputs.values.foreach(e => {
      val aoConfig = new AnalogOutputStatusConfig()
      aoConfig.staticVariation = StaticAnalogOutputStatusVariation.GROUP40_VAR2

      db.addAnalogOutputStatus(e.index, io.stepfunc.dnp3.EventClass.CLASS1, aoConfig)
      db.updateAnalogOutputStatus(e, updateOptions)
    })
  })

  def generateBinaryInputEvent(idx: Int, eventBatch: Int): TypedEvent[Binary] = {
    val value = !binaryPoints(idx).value
    val flags = if(value) ubyte(0x81) else ubyte(0x01)
    val newPoint = new Binary(ushort(idx), value, new Flags(flags), app.now())

    this.binaryPoints(idx) = newPoint
    val newEvent = new TypedEvent[Binary](idx, eventBatch, EventClass.Class1, newPoint)
    this.events.append(newEvent)

    outstation.transaction(db => {
      db.updateBinary(newPoint, new UpdateOptions)
    })

    newEvent
  }

  def generateDoubleBitBinaryInputEvent(idx: Int, eventBatch: Int): TypedEvent[DoubleBitBinary] = {
    val value = if (doubleBitPoints(idx).value == DoubleBit.DETERMINED_OFF) DoubleBit.DETERMINED_ON else DoubleBit.DETERMINED_OFF
    val flags = if (value == DoubleBit.DETERMINED_OFF) ubyte(0x41) else ubyte(0x81)
    val newPoint = new DoubleBitBinary(ushort(idx), value, new Flags(flags), app.now())

    this.doubleBitPoints(idx) = newPoint
    val newEvent = new TypedEvent[DoubleBitBinary](idx, eventBatch, EventClass.All, newPoint)
    this.events.append(newEvent)

    outstation.transaction(db => {
      db.updateDoubleBitBinary(newPoint, new UpdateOptions)
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
      db.updateCounter(newPoint, new UpdateOptions)
    })

    newEvent
  }

  def recordCurrentCounterValues(): Unit = {
    this.recordedCounterValues = this.counters.clone()
  }

  def generateAnalogInputEvent(idx: Int, eventBatch: Int): TypedEvent[Analog] = {
    val value = analogInputs(idx).value + 100.0
    val newPoint = new Analog(ushort(idx), value, new Flags(ubyte(0x01)), app.now())

    this.analogInputs(idx) = newPoint
    val newEvent = new TypedEvent[Analog](idx, eventBatch, EventClass.Class3, newPoint)
    this.events.append(newEvent)

    outstation.transaction(db => {
      db.updateAnalog(newPoint, new UpdateOptions)
    })

    newEvent
  }

  def resetHandledEvents(): Unit = {
    this.events.foreach(e => e.reset())
  }

  def getAllPoints(isClass0: Boolean): Set[Event] = {
    (
      getAllBinaryInputs.map(e => e.asInstanceOf[Event]).toList :::
      (if (!isClass0) getAllDoubleBitBinaryInputs.map(_.asInstanceOf[Event]).toList else Nil) :::
      getAllCounters.map(_.asInstanceOf[Event]).toList :::
      getAllCounters.map(e => new TypedEvent[FrozenCounter](e.idx, e.batch, EventClass.All, new FrozenCounter(e.value.index, uint(0), e.value.flags, e.value.time)).asInstanceOf[Event]).toList :::
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

  def getAllBinaryInputs: Seq[TypedEvent[Binary]] = {
    this.binaryPoints.map(e => {
      new TypedEvent(e._1, 0, EventClass.All, e._2)
    }).toSeq
  }

  def getAllBinaryInputEvents(batch: BatchSpecifier): Seq[TypedEvent[Binary]] = {
    getAllEvents(batch).flatMap {
      case event @ TypedEvent(_: Binary) => Some(event.asInstanceOf[TypedEvent[Binary]])
      case _ => None
    }
  }

  def getUnhandledBinaryInputEvents(batch: BatchSpecifier): Seq[TypedEvent[Binary]] = {
    getAllBinaryInputEvents(batch).filter(e => !e.isHandled)
  }

  def popBinaryEvent(batch: BatchSpecifier): Option[TypedEvent[Binary]] = {
    val head = getUnhandledBinaryInputEvents(batch).headOption
    head match {
      case Some(event) => event.handle()
      case None =>
    }
    head
  }

  def getAllDoubleBitBinaryInputs: Seq[TypedEvent[DoubleBitBinary]] = {
    this.doubleBitPoints.map(e => {
      new TypedEvent(e._1, 0, EventClass.All, e._2)
    }).toSeq
  }

  def getAllDoubleBitBinaryInputEvents(batch: BatchSpecifier): Seq[TypedEvent[DoubleBitBinary]] = {
    getAllEvents(batch).flatMap {
      case event @ TypedEvent(_: DoubleBitBinary) => Some(event.asInstanceOf[TypedEvent[DoubleBitBinary]])
      case _ => None
    }
  }

  def getUnhandledDoubleBitBinaryInputEvents(batch: BatchSpecifier): Seq[TypedEvent[DoubleBitBinary]] = {
    getAllDoubleBitBinaryInputEvents(batch).filter(e => !e.isHandled)
  }

  def popDoubleBitBinaryEvent(batch: BatchSpecifier): Option[TypedEvent[DoubleBitBinary]] = {
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

  def getAllAnalogInputs: Seq[TypedEvent[Analog]] = {
    this.analogInputs.map(e => {
      new TypedEvent(e._1, 0, EventClass.All, e._2)
    }).toSeq
  }

  def getAllAnalogInputEvents(batch: BatchSpecifier): Seq[TypedEvent[Analog]] = {
    getAllEvents(batch).flatMap {
      case event @ TypedEvent(_: Analog) => Some(event.asInstanceOf[TypedEvent[Analog]])
      case _ => None
    }
  }

  def getUnhandledAnalogInputEvents(batch: BatchSpecifier): Seq[TypedEvent[Analog]] = {
    getAllAnalogInputEvents(batch).filter(e => !e.isHandled)
  }

  def popAnalogInputEvent(batch: BatchSpecifier): Option[TypedEvent[Analog]] = {
    val head = getUnhandledAnalogInputEvents(batch).headOption
    head match {
      case Some(event) => event.handle()
      case None =>
    }
    head
  }
}
