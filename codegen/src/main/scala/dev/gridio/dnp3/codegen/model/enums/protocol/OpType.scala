package dev.gridio.dnp3.codegen.model.enums.protocol

import dev.gridio.dnp3.codegen.model.{EnumModel, EnumValue, IntRender}

object OpType extends EnumModel {
  override def name: String = "OpType"
  override def comments: List[String] = List("Used in conjunction with the TCC field to specify a control operation")

  override def values: List[EnumValue] = List(
    EnumValue("Nul", 0, None, None),
    EnumValue("PulseOn", 1, None, None),
    EnumValue("PulseOff", 2, None, None),
    EnumValue("LatchOn", 3, None, None),
    EnumValue("LatchOff", 4, None, None),
  )

  override def render: IntRender = IntRender.Base10

  override def captureUnknownValues: Boolean = true
}
