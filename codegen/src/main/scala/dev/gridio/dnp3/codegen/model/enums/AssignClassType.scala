package dev.gridio.dnp3.codegen.model.enums

import dev.gridio.dnp3.codegen.model.{EnumModel, EnumValues, Hex}


object AssignClassType {

  private val comments = List(
    "groups that can be used in conjunction with the ASSIGN_CLASS function code"
  )

  def apply(): EnumModel = EnumModel("AssignClassType", comments, EnumModel.UInt8, codes, None, Hex)

  private val codes = EnumValues.from(List(
    "BinaryInput",
    "DoubleBinaryInput",
    "Counter",
    "FrozenCounter",
    "AnalogInput",
    "BinaryOutputStatus",
    "AnalogOutputStatus"
  ))

}



