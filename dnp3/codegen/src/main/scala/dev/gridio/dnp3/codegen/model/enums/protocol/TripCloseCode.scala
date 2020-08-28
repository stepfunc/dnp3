package dev.gridio.dnp3.codegen.model.enums.protocol

import dev.gridio.dnp3.codegen.model.{EnumModel, EnumValue, IntRender}

object TripCloseCode extends EnumModel {
  override def name: String = "TripCloseCode"

  override def comments: List[String] = List("Field is used in conjunction with the `OpType` field to specify a control operation")

  override def values: List[EnumValue] = List(
    EnumValue("Nul", 0, "not specified"),
    EnumValue("Close", 1, "close output"),
    EnumValue("Trip", 2, "trip output"),
    EnumValue("Reserved", 3, "reserved for future use"),
  )

  override def render: IntRender = IntRender.Base10

  override def captureUnknownValues: Boolean = true
}
