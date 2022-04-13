package io.stepfunc.conformance.dnp3

import io.stepfunc.dnp3._
import org.joou.UShort
import org.joou.Unsigned.{ubyte, uint, ushort}

import java.util.concurrent.ConcurrentLinkedDeque

class QueuedControlHandler(val binaryOutputsDisabled: Boolean, val analogOutputsDisabled: Boolean) extends ControlHandler {
  val waitTime = 1000

  private val binaryOutputCommands = new ConcurrentLinkedDeque[Integer]()
  private val analogOutputCommands = new ConcurrentLinkedDeque[Integer]()

  override def beginFragment(): Unit = {}

  override def endFragment(database: DatabaseHandle): Unit = {}

  override def selectG12v1(control: Group12Var1, index: UShort, database: DatabaseHandle): CommandStatus = {
    checkBinaryOutputCommand(control, index, operate = false)
  }

  override def selectG41v1(control: Int, index: UShort, database: DatabaseHandle): CommandStatus = {
    checkAnalogOutputCommand(control.toDouble, index, operate = false, database)
  }

  override def selectG41v2(control: Short, index: UShort, database: DatabaseHandle): CommandStatus = {
    checkAnalogOutputCommand(control.toDouble, index, operate = false, database)
  }

  override def selectG41v3(control: Float, index: UShort, database: DatabaseHandle): CommandStatus = {
    checkAnalogOutputCommand(control.toDouble, index, operate = false, database)
  }

  override def selectG41v4(control: Double, index: UShort, database: DatabaseHandle): CommandStatus = {
    checkAnalogOutputCommand(control, index, operate = false, database)
  }

  override def operateG12v1(control: Group12Var1, index: UShort, opType: OperateType, database: DatabaseHandle): CommandStatus = {
    checkBinaryOutputCommand(control, index, operate = true)
  }

  override def operateG41v1(control: Int, index: UShort, opType: OperateType, database: DatabaseHandle): CommandStatus = {
    checkAnalogOutputCommand(control.toDouble, index, operate = true, database)
  }

  override def operateG41v2(control: Short, index: UShort, opType: OperateType, database: DatabaseHandle): CommandStatus = {
    checkAnalogOutputCommand(control.toDouble, index, operate = true, database)
  }

  override def operateG41v3(control: Float, index: UShort, opType: OperateType, database: DatabaseHandle): CommandStatus = {
    checkAnalogOutputCommand(control.toDouble, index, operate = true, database)
  }

  override def operateG41v4(control: Double, index: UShort, opType: OperateType, database: DatabaseHandle): CommandStatus = {
    checkAnalogOutputCommand(control, index, operate = true, database)
  }

  def checkCrob(index: Int): Unit = {
    val startTime = System.currentTimeMillis()

    while(System.currentTimeMillis() - startTime < waitTime) {
      val value = binaryOutputCommands.pollFirst()
      if(value != null) {
        if(value == index) return
        else throw new Exception(s"Unexpected operate Binary Output $value ($index expected)")
      }
      Thread.sleep(100)
    }

    throw new Exception(s"Binary Output $index never operated")
  }

  def checkNoCrob(): Unit = {
    val startTime = System.currentTimeMillis()

    while(System.currentTimeMillis() - startTime < waitTime) {
      val value = binaryOutputCommands.pollFirst()
      if(value != null) {
        throw new Exception(s"Unexpected operate on Binary Output $value (none expected)")
      }
      Thread.sleep(100)
    }
  }

  def checkAnalog(index: Int): Unit = {
    val startTime = System.currentTimeMillis()

    while(System.currentTimeMillis() - startTime < waitTime) {
      val value = analogOutputCommands.pollFirst()
      if(value != null) {
        if(value == index) return
        else throw new Exception(s"Unexpected operate Analog Output $value ($index expected)")
      }
      Thread.sleep(100)
    }

    throw new Exception(s"Analog Output $index never operated")
  }

  def checkNoAnalog(): Unit = {
    val startTime = System.currentTimeMillis()

    while(System.currentTimeMillis() - startTime < waitTime) {
      val value = analogOutputCommands.pollFirst()
      if(value != null) {
        throw new Exception(s"Unexpected operate on Analog Output $value (none expected)")
      }
      Thread.sleep(100)
    }
  }

  private def checkBinaryOutputCommand(crob: Group12Var1, index: UShort, operate: Boolean) : CommandStatus = {
    if (binaryOutputsDisabled) return CommandStatus.NOT_SUPPORTED

    val result = index.intValue() match {
      case 0 =>
        if(crob.code.opType == OpType.LATCH_ON &&
          crob.code.tcc == TripCloseCode.NUL &&
          !crob.code.clear &&
          crob.count == ubyte(1) &&
          crob.onTime == uint(100) &&
          crob.offTime == uint(200)) {
          CommandStatus.SUCCESS
        }
        else CommandStatus.FORMAT_ERROR
      case 1 =>
        if(crob.code.opType == OpType.PULSE_ON &&
          crob.code.tcc == TripCloseCode.TRIP &&
          !crob.code.clear &&
          crob.count == ubyte(5) &&
          crob.onTime == uint(300) &&
          crob.offTime == uint(400)) {
          CommandStatus.SUCCESS
        }
        else CommandStatus.FORMAT_ERROR
      case 2 =>
        if((crob.code.opType == OpType.LATCH_ON || crob.code.opType == OpType.LATCH_OFF) &&
          crob.code.tcc == TripCloseCode.NUL &&
          !crob.code.clear &&
          crob.count == ubyte(10) &&
          crob.onTime == uint(500) &&
          crob.offTime == uint(600)) {
          CommandStatus.SUCCESS
        }
        else CommandStatus.FORMAT_ERROR
      case 3 =>
        if(crob.code.opType == OpType.PULSE_ON &&
          (crob.code.tcc == TripCloseCode.TRIP || crob.code.tcc == TripCloseCode.CLOSE) &&
          !crob.code.clear &&
          crob.count == ubyte(15) &&
          crob.onTime == uint(700) &&
          crob.offTime == uint(800)) {
          CommandStatus.SUCCESS
        }
        else CommandStatus.FORMAT_ERROR
      case 4 =>
        if(((crob.code.opType == OpType.LATCH_ON && crob.code.tcc == TripCloseCode.NUL) || (crob.code.opType == OpType.LATCH_OFF && crob.code.tcc == TripCloseCode.NUL) || (crob.code.opType == OpType.PULSE_ON && crob.code.tcc == TripCloseCode.TRIP) || (crob.code.opType == OpType.PULSE_ON && crob.code.tcc == TripCloseCode.CLOSE)) &&
          !crob.code.clear &&
          crob.count == ubyte(20) &&
          crob.onTime == uint(900) &&
          crob.offTime == uint(1000)) {
          CommandStatus.SUCCESS
        }
        else CommandStatus.FORMAT_ERROR
      case 5 =>
        if(crob.code.opType == OpType.PULSE_ON &&
          crob.code.tcc == TripCloseCode.NUL &&
          !crob.code.clear &&
          crob.count == ubyte(25) &&
          crob.onTime == uint(1100) &&
          crob.offTime == uint(1200)) {
          CommandStatus.SUCCESS
        }
        else CommandStatus.FORMAT_ERROR
      case 6 =>
        if(crob.code.opType == OpType.LATCH_ON &&
          crob.code.tcc == TripCloseCode.NUL &&
          !crob.code.clear &&
          crob.count == ubyte(30) &&
          crob.onTime == uint(1300) &&
          crob.offTime == uint(1400)) {
          CommandStatus.SUCCESS
        }
        else CommandStatus.FORMAT_ERROR
      case 7 =>
        if(crob.code.opType == OpType.LATCH_OFF &&
          crob.code.tcc == TripCloseCode.NUL &&
          !crob.code.clear &&
          crob.count == ubyte(35) &&
          crob.onTime == uint(1500) &&
          crob.offTime == uint(1600)) {
          CommandStatus.SUCCESS
        }
        else CommandStatus.FORMAT_ERROR
      case 8 =>
        if(crob.code.opType == OpType.PULSE_ON &&
          crob.code.tcc == TripCloseCode.CLOSE &&
          !crob.code.clear &&
          crob.count == ubyte(40) &&
          crob.onTime == uint(1700) &&
          crob.offTime == uint(1800)) {
          CommandStatus.SUCCESS
        }
        else CommandStatus.FORMAT_ERROR
      case 9 =>
        if(crob.code.opType == OpType.PULSE_ON &&
          crob.code.tcc == TripCloseCode.TRIP &&
          !crob.code.clear &&
          crob.count == ubyte(45) &&
          crob.onTime == uint(1900) &&
          crob.offTime == uint(2000)) {
          CommandStatus.SUCCESS
        }
        else CommandStatus.FORMAT_ERROR
      case i if i.intValue() >= 10 && i.intValue() <= 19 =>
        if(crob.code.opType == OpType.LATCH_ON &&
          crob.code.tcc == TripCloseCode.NUL &&
          !crob.code.clear &&
          crob.count == ubyte(1) &&
          crob.onTime == uint(100) &&
          crob.offTime == uint(200)) {
          CommandStatus.SUCCESS
        }
        else CommandStatus.FORMAT_ERROR
      case _ => CommandStatus.NOT_SUPPORTED
    }

    if(operate && result == CommandStatus.SUCCESS) {
      binaryOutputCommands.add(index.intValue())
    }

    result
  }

  private def checkAnalogOutputCommand(value: Double, index: UShort, operate: Boolean, database: DatabaseHandle) : CommandStatus = {
    if (analogOutputsDisabled) return CommandStatus.NOT_SUPPORTED

    val result = index.intValue() match {
      case 0 =>
        if(value == 10.0) CommandStatus.SUCCESS else CommandStatus.FORMAT_ERROR
      case 1 => if(value == 20.0) CommandStatus.SUCCESS else CommandStatus.FORMAT_ERROR
      case i if i.intValue() >= 10 && i.intValue() <= 19 => if(value == 10.0) CommandStatus.SUCCESS else CommandStatus.FORMAT_ERROR
      case _ => CommandStatus.NOT_SUPPORTED
    }

    if(operate && result == CommandStatus.SUCCESS) {
      analogOutputCommands.add(index.intValue())

      // Update the associated Analog Output Status
      val flags = new Flags(Flag.ONLINE)
      database.transaction(db => db.updateAnalogOutputStatus(new AnalogOutputStatus(index, value, flags, Timestamp.invalidTimestamp()), UpdateOptions.detectEvent()))
    }

    result
  }
}
