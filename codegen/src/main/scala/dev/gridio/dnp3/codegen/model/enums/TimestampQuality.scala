package dev.gridio.dnp3.codegen.model.enums

import dev.gridio.dnp3.codegen.model.{Base10, EnumModel, EnumValue}

object TimestampQuality {

  private val comments = List("Indicates the quality of timestamp values")

  val defaultValue = EnumValue("INVALID", 0, "Timestamp is not valid, ignore the value and use a local timestamp")

  def apply(): EnumModel = EnumModel("TimestampQuality", comments, EnumModel.UInt8, codes, Some(defaultValue), Base10)

  private val codes = List(
    EnumValue("SYNCHRONIZED", 1, "The timestamp is UTC synchronized at the remote device"),
    EnumValue("UNSYNCHRONIZED", 2, "The device indicate the timestamp may be unsynchronized")
  )

}



