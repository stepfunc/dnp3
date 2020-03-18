package dev.gridio.dnp3.codegen.model.enums

import dev.gridio.dnp3.codegen.model
import dev.gridio.dnp3.codegen.model.{EnumModel, EnumValue, Hex}

object DoubleBit {

  private val comments = List("Enumeration for possible states of a double bit value")

  def apply(): EnumModel = model.EnumModel("DoubleBit", comments, EnumModel.UInt8, codes, Some(defaultValue), Hex)

  def defaultValue: EnumValue = EnumValue("INDETERMINATE", 3, "Abnormal or custom condition")

  private val codes = List(
    EnumValue("INTERMEDIATE", 0, "Transitioning between end conditions"),
    EnumValue("DETERMINED_OFF", 1, "End condition, determined to be OFF"),
    EnumValue("DETERMINED_ON", 2, "End condition, determined to be ON")
  )

}



