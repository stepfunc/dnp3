package dev.gridio.dnp3.codegen.model.enums

import dev.gridio.dnp3.codegen.model.{Base10, EnumModel, EnumValue}

object LinkStatus {

  private val comments = List("Enumeration for reset/unreset states of a link layer")

  def apply(): EnumModel = EnumModel("LinkStatus", comments, EnumModel.UInt8, codes, None, Base10)

  private val codes = List(
    EnumValue("UNRESET", 0, "DOWN"),
    EnumValue("RESET", 1, "UP")
  )

}



