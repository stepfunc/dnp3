package dev.gridio.dnp3.codegen.model.enums.protocol

import dev.gridio.dnp3.codegen.model.{EnumModel, EnumValue, IntRender}

object OpType extends EnumModel {
  override def name: String = "OpType"
  override def comments: List[String] = List("Field used in conjunction with the `TCC` field to specify a control operation")

  override def values: List[EnumValue] = List(
    EnumValue("Nul", 0, "not specified"),
    EnumValue("PulseOn", 1, "pulse the output on"),
    EnumValue("PulseOff", 2, "pulse the output off"),
    EnumValue("LatchOn", 3, "latch the output on"),
    EnumValue("LatchOff", 4, "latch the output off"),
  )

  override def render: IntRender = IntRender.Base10

  override def captureUnknownValues: Boolean = true
}
