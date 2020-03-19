package dev.gridio.dnp3.codegen.model.enums

import dev.gridio.dnp3.codegen.model.{EnumModel, EnumValue, Hex, ObjectGroup}

object GroupVariationEnum {

  private val defaultValue = EnumValue("UNKNOWN", 0xFFFF)

  private def values: List[EnumValue] = ObjectGroup.all.flatMap(_.variations).map { gv =>
    EnumValue(gv.name, gv.shortValue, None, Some(gv.fullDesc))
  }

  def apply(): EnumModel = EnumModel(
    "GroupVariation",
    List("Comprehensive list of supported groups and variations"),
    EnumModel.UInt16,
    values,
    Some(defaultValue),
    Hex
  )

}
