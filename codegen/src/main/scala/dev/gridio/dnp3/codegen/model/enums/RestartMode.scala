package dev.gridio.dnp3.codegen.model.enums

import dev.gridio.dnp3.codegen.model.{Base10, EnumModel, EnumValue}

object RestartMode {

  private val comments = List("Enumeration describing restart mode support of an outstation")

  def apply(): EnumModel = EnumModel("RestartMode", comments, EnumModel.UInt8, codes, None, Base10)

  private val codes = List(
    EnumValue("UNSUPPORTED", 0, "Device does not support restart"),
    EnumValue("SUPPORTED_DELAY_FINE", 1, "Supports restart, and time returned is a fine time delay"),
    EnumValue("SUPPORTED_DELAY_COARSE", 2, "Supports restart, and time returned is a coarse time delay")
  )

}



