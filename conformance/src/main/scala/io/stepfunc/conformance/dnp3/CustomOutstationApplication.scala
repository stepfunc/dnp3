package io.stepfunc.conformance.dnp3

import io.stepfunc.dnp3._
import org.joou.Unsigned.{ulong, ushort, uint}
import org.joou.{ULong, UShort}

import java.time.{Duration, Instant}

class CustomOutstationApplication(val isLocalControl: Boolean) extends OutstationApplication {
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

  override def coldRestart: RestartDelay = RestartDelay.milliseconds(ushort(5000))

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

  private def isTimeValid: Boolean = {
    Duration.between(this.lastUpdate, Instant.now).compareTo(refreshRate) <= 0
  }

  private def needsTime: Boolean = {
    Duration.between(this.lastUpdate, Instant.now).compareTo(refreshRate.dividedBy(2)) > 0
  }


}
