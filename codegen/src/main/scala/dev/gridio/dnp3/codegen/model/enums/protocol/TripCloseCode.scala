package dev.gridio.dnp3.codegen.model.enums.protocol

import dev.gridio.dnp3.codegen.model.{EnumModel, EnumValue, IntRender}

object TripCloseCode extends EnumModel {
  override def name: String = "TripCloseCode"

  override def comments: List[String] = List("This field is used in conjunction with the Op Type field to specify a control operation")

  override def values: List[EnumValue] = List(
    EnumValue("Nul", 0, None, None),
    EnumValue("Close", 1, None, None),
    EnumValue("Trip", 2, None, None),
    EnumValue("Reserved", 3, None, None),
  )

  override def render: IntRender = IntRender.Base10

  override def captureUnknownValues: Boolean = true
}
