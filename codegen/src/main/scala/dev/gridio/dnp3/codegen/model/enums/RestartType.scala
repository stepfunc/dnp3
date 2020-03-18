package dev.gridio.dnp3.codegen.model.enums

import dev.gridio.dnp3.codegen.model.{Base10, EnumModel, EnumValue}

object RestartType {

  private val comments = List("Enumeration describing restart operation to perform on an outstation")

  def apply(): EnumModel = EnumModel("RestartType", comments, EnumModel.UInt8, codes, None, Base10)

  private val codes = List(
    EnumValue("COLD", 0, "Full reboot"),
    EnumValue("WARM", 1, "Warm reboot of process only")
  )

}



