package dev.gridio.dnp3.codegen.model.enums

import dev.gridio.dnp3.codegen.model.{EnumModel, Hex}


object StaticTypeBitmask {

  private val comments = List(
    "Bitmask values for all the static types"
  )

  def apply(): EnumModel = EnumModel("StaticTypeBitmask", comments, EnumModel.UInt16, EnumModel.BitfieldValues(names), None, Hex)

  private val names = List(
    "BinaryInput",
    "DoubleBinaryInput",
    "Counter",
    "FrozenCounter",
    "AnalogInput",
    "BinaryOutputStatus",
    "AnalogOutputStatus",
    "TimeAndInterval",
    "OctetString"
  )

}



