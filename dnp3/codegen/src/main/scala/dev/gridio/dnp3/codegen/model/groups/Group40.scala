// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._
import dev.gridio.dnp3.codegen.model._

// Analog output status
object Group40 extends ObjectGroup {
  def variations: List[Variation] = List(Group40Var0, Group40Var1, Group40Var2, Group40Var3, Group40Var4)

  def group: Byte = 40

  def desc: String = "Analog Output Status"

  override def groupType: GroupType = GroupType.StaticAnalogOutputStatus
}

object Group40Var0 extends AnyVariation(Group40, 0)

object Group40Var1 extends FixedSize(Group40, 1, bit32WithFlags)(flags, value32)

object Group40Var2 extends FixedSize(Group40, 2, bit16WithFlags)(flags, value16)

object Group40Var3 extends FixedSize(Group40, 3, singlePrecisionWithFlags)(flags, float32)

object Group40Var4 extends FixedSize(Group40, 4, doublePrecisionWithFlags)(flags, float64)
