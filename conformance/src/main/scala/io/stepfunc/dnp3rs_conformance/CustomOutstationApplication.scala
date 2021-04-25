package io.stepfunc.dnp3_conformance

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

  override def coldRestart: RestartDelay = RestartDelay.validMillis(ushort(5000))

  override def warmRestart: RestartDelay = RestartDelay.notSupported()

  override def freezeCountersAll(freezeType: FreezeType, database: Database): FreezeResult = {
    for (i <- 0 to 9) {
      val currentCounter = database.getCounter(ushort(i))
      database.updateFrozenCounter(new FrozenCounter(currentCounter.index, currentCounter.value, currentCounter.flags, currentCounter.time), new UpdateOptions())
      if (freezeType == FreezeType.FREEZE_AND_CLEAR) {
        currentCounter.value = uint(0)
        database.updateCounter(currentCounter, new UpdateOptions())
      }
    }

    FreezeResult.SUCCESS
  }

  override def freezeCountersRange(start: UShort, stop: UShort, freezeType: FreezeType, database: Database): FreezeResult = {
    if (start.intValue() > 9 || stop.intValue() > 9) {
      return FreezeResult.PARAMETER_ERROR
    }

    for (i <- start.intValue() to stop.intValue()) {
      val currentCounter = database.getCounter(ushort(i))
      database.updateFrozenCounter(new FrozenCounter(currentCounter.index, currentCounter.value, currentCounter.flags, currentCounter.time), new UpdateOptions())
      if (freezeType == FreezeType.FREEZE_AND_CLEAR) {
        currentCounter.value = uint(0)
        database.updateCounter(currentCounter, new UpdateOptions())
      }
    }

    FreezeResult.SUCCESS
  }

  def now(): Timestamp = {
    val time = this.lastTimestamp.plus(Duration.between(this.lastUpdate, Instant.now))
    val quality = if (isTimeValid) TimeQuality.SYNCHRONIZED else TimeQuality.NOT_SYNCHRONIZED
    new Timestamp(ulong(time.toEpochMilli), quality)
  }

  private def isTimeValid: Boolean = {
    Duration.between(this.lastUpdate, Instant.now).compareTo(refreshRate) <= 0
  }

  private def needsTime: Boolean = {
    Duration.between(this.lastUpdate, Instant.now).compareTo(refreshRate.dividedBy(2)) > 0
  }


}
