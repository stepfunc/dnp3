package dev.gridio.dnp3.codegen.model.enums

import dev.gridio.dnp3.codegen.model
import dev.gridio.dnp3.codegen.model.{Base10, EnumModel, EnumValue}

object Parity {

  private val comments = List("Enumeration for setting serial port parity")

  def apply(): EnumModel = model.EnumModel("Parity", comments, EnumModel.UInt8, codes, Some(default), Base10)

  def default = EnumValue("None", 0)

  private val codes = List(
    EnumValue("Even", 1),
    EnumValue("Odd", 2)
  )

}

object FlowControl {

  private val comments = List("Enumeration for setting serial port flow control")

  def apply(): EnumModel = model.EnumModel("FlowControl", comments, EnumModel.UInt8, codes, Some(default), Base10)

  def default = EnumValue("None", 0)

  private val codes = List(
    EnumValue("Hardware", 1),
    EnumValue("XONXOFF", 2)
  )

}


object StopBits {

  private val comments = List("Enumeration for setting serial port stop bits")

  def apply(): EnumModel = model.EnumModel("StopBits", comments, EnumModel.UInt8, codes, Some(default), Base10)

  def default = EnumValue("None", 0)

  private val codes = List(
    EnumValue("One", 1),
    EnumValue("OnePointFive", 2),
    EnumValue("Two", 3)
  )

}
