package dev.gridio.dnp3.codegen.model.enums

import dev.gridio.dnp3.codegen.model.{EnumModel, EnumValues, Hex}


object LogLevel {

  private val comments = List("Enumeration for log levels")

  def apply(): EnumModel = EnumModel("LogLevel", comments, EnumModel.UInt32, codes, None, Hex)

  private val codes = EnumValues.bitmask(levels)

  private def levels = List("Event", "Error", "Warning", "Info", "Interpret", "Comm", "Debug")


}



