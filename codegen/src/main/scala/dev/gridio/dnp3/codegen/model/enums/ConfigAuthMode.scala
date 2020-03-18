package dev.gridio.dnp3.codegen.model.enums

import dev.gridio.dnp3.codegen.model.{EnumModel, EnumValue, Hex}

object ConfigAuthMode {

  private val comments = List(
    "Configuration enum for the authentication mode"
  )

  def apply(): EnumModel = EnumModel("ConfigAuthMode", comments, EnumModel.UInt8, codes, None, Hex)

  private val codes = List(
    EnumValue("NONE", 0, "No authentication"),
    EnumValue("SAV5", 1, "Secure authentication version 5")
  )

}



