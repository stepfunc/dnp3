// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package io.stepfunc.conformance.dnp3

import io.stepfunc.dnp3._
import org.joou.Unsigned.{ulong, ushort, uint}
import org.joou.{UByte, ULong, UShort}

import java.time.{Duration, Instant}

class CustomOutstationApplication extends OutstationApplication {
  @volatile var isLocalControl = false
  // called when the master writes a writable string attribute
  @volatile var onWriteStringAttribute: (Int, String) => Unit = (_, _) => ()
  // called for each point header of an accepted ASSIGN_CLASS request, with its range, if any
  @volatile var onAssignClass: (PointType, Option[(Int, Int)], EventClass) => Unit = (_, _, _) => ()
  private val refreshRate: Duration = Duration.ofSeconds(10)
  private var lastTimestamp = Instant.MIN
  private var lastUpdate = Instant.MIN

  override def getProcessingDelayMs: UShort = ushort(0)

  override def writeAbsoluteTime(time: ULong): WriteTimeResult = {
    this.lastTimestamp = Instant.ofEpochMilli(time.longValue())
    this.lastUpdate = Instant.now
    WriteTimeResult.OK
  }

  override def getApplicationIin: ApplicationIin = {
    val iin = new ApplicationIin()
    iin.localControl = isLocalControl
    iin.needTime = needsTime
    iin
  }

  override def writeStringAttr(set: UByte, variation: UByte, attrType: StringAttr, value: String): Boolean = {
    onWriteStringAttribute(variation.intValue, value)
    true
  }

  override def assignClassAll(pointType: PointType, pointClass: EventClass): Unit =
    onAssignClass(pointType, None, pointClass)

  override def assignClassRange(pointType: PointType, start: UShort, stop: UShort, pointClass: EventClass): Unit =
    onAssignClass(pointType, Some((start.intValue, stop.intValue)), pointClass)

  // whether the outstation supports cold restart (3.1r1's 8.8.2)
  @volatile var coldRestartSupported = true

  override def coldRestart: RestartDelay =
    if (coldRestartSupported) RestartDelay.milliseconds(ushort(5000)) else RestartDelay.notSupported()

  override def warmRestart: RestartDelay = RestartDelay.notSupported()

  override def freezeCountersAll(freezeType: FreezeType, database: DatabaseHandle): FreezeResult = {
    database.transaction(db => {
      for (i <- 0 to 9) {
        val currentCounter = db.getCounter(ushort(i))
        db.updateFrozenCounter(new FrozenCounter(currentCounter.index, currentCounter.value, currentCounter.flags, currentCounter.time), UpdateOptions.detectEvent())
        if (freezeType == FreezeType.FREEZE_AND_CLEAR) {
          currentCounter.value = uint(0)
          db.updateCounter(currentCounter, UpdateOptions.detectEvent())
        }
      }
    })


    FreezeResult.OK
  }

  override def freezeCountersRange(start: UShort, stop: UShort, freezeType: FreezeType, database: DatabaseHandle): FreezeResult = {
    database.transaction(db => {
      if (start.intValue() > 9 || stop.intValue() > 9) {
        return FreezeResult.PARAMETER_ERROR
      }

      for (i <- start.intValue() to stop.intValue()) {
        val currentCounter = db.getCounter(ushort(i))
        db.updateFrozenCounter(new FrozenCounter(currentCounter.index, currentCounter.value, currentCounter.flags, currentCounter.time), UpdateOptions.detectEvent())
        if (freezeType == FreezeType.FREEZE_AND_CLEAR) {
          currentCounter.value = uint(0)
          db.updateCounter(currentCounter, UpdateOptions.detectEvent())
        }
      }
    })

    FreezeResult.OK
  }

  def now(): Timestamp = {
    val time = ulong(this.lastTimestamp.plus(Duration.between(this.lastUpdate, Instant.now)).toEpochMilli)
    if (isTimeValid) Timestamp.synchronizedTimestamp(time) else Timestamp.unsynchronizedTimestamp(time)
  }

  def isTimeValid: Boolean = {
    Duration.between(this.lastUpdate, Instant.now).compareTo(refreshRate) <= 0
  }

  private def needsTime: Boolean = {
    Duration.between(this.lastUpdate, Instant.now).compareTo(refreshRate.dividedBy(2)) > 0
  }


}
