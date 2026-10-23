// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.VariationNames._
import dev.gridio.dnp3.codegen.model._

object Group1 extends ObjectGroup {

  def variations: List[Variation] = List(Group1Var0, Group1Var1, Group1Var2)

  def desc: String = "Binary Input"

  def group: Byte = 1

  override def groupType: GroupType = GroupType.StaticBinary
}

object Group1Var0 extends AnyVariation(Group1, 0)

object Group1Var1 extends SingleBitField(Group1, 1, packedFormat)
object Group1Var2 extends FixedSize(Group1, 2, withFlags)(FixedSizeField.flags)

