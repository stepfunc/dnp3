// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._

object Group3 extends ObjectGroup {
  def variations: List[Variation] = List(Group3Var0, Group3Var1, Group3Var2)

  def group: Byte = 3

  def desc: String = "Double-bit Binary Input"

  override def groupType: GroupType = GroupType.StaticDoubleBinary
}

object Group3Var0 extends AnyVariation(Group3, 0)

object Group3Var1 extends DoubleBitField(Group3, 1, packedFormat)

object Group3Var2 extends FixedSize(Group3, 2, withFlags)(flags)
