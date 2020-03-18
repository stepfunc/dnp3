package dev.gridio.dnp3.codegen.model.enums

import dev.gridio.dnp3.codegen.model.{EnumModel, EnumValue, Hex}

object KeyWrapAlgorithm {

  private val comments = List("Enumerates possible key-wrap algorithms")
  private val defaultValue = EnumValue("UNDEFINED", 0)

  def apply(): EnumModel = EnumModel("KeyWrapAlgorithm", comments, EnumModel.UInt8, codes, Some(defaultValue), Hex)

  private val codes = List(
    EnumValue("AES_128", 1, "AES 128 Key Wrap Algorithm"),
    EnumValue("AES_256", 2, "AES 256 Key Wrap Algorithm")
  )

}



