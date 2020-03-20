package dev.gridio.dnp3.codegen.model.enums.protocol

import dev.gridio.dnp3.codegen.model._

object QualifierCode extends EnumModel {

  override def captureUnknownValues: Boolean = false

  override def render: IntRender = IntRender.Hex

  override def name: String = "QualifierCode"

  override def comments: List[String] =  List("application object header types")

  override def values: List[EnumValue] = List(
    EnumValue("Range8", 0x00, None, Some("8-bit start stop")),
    EnumValue("Range16", 0x01, None, Some("16-bit start stop")),
    EnumValue("AllObjects", 0x06, None, Some("all objects")),
    EnumValue("Count8", 0x07, None, Some("8-bit count")),
    EnumValue("Count16", 0x08, None, Some("16-bit count")),
    EnumValue("CountAndPrefix8", 0x17, None, Some("8-bit count and prefix")),
    EnumValue("CountAndPrefix16", 0x28, None, Some("16-bit count and prefix")),
    EnumValue("FreeFormat16", 0x5B, None, Some("16-bit free format"))
  )

}
