// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

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
