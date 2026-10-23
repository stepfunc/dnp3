// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._
import dev.gridio.dnp3.codegen.model._

object Group10 extends ObjectGroup {
  def variations: List[Variation] = List(Group10Var0, Group10Var1, Group10Var2)

  def desc: String = "Binary Output"

  def group: Byte = 10

  override def groupType: GroupType = GroupType.StaticBinaryOutputStatus
}

object Group10Var0 extends AnyVariation(Group10, 0)

object Group10Var1 extends SingleBitField(Group10, 1, packedFormat)

object Group10Var2 extends FixedSize(Group10, 2, withFlags)(flags)

