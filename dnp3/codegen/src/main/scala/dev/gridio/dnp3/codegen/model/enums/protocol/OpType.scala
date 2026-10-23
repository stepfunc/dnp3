// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

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
