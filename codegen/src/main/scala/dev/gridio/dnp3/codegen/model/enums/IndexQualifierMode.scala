package dev.gridio.dnp3.codegen.model.enums

import dev.gridio.dnp3.codegen.model.{EnumModel, EnumValue, Hex}

object IndexQualifierMode {

  private val comments = List("Specifies whether opendnp3 optimizes for 1-byte indexes when making requests")

  def apply(): EnumModel = EnumModel("IndexQualifierMode", comments, EnumModel.UInt8, codes, None, Hex)

  private val codes = List(
    EnumValue("allow_one_byte", 0, "Use a one byte qualifier if possible"),
    EnumValue("always_two_bytes", 1, "Always use two byte qualifiers even if the index is less than or equal to 255")
  )

}



