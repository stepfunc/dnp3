// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._
import dev.gridio.dnp3.codegen.model._

// counters
object Group20 extends ObjectGroup {
  def variations: List[Variation] = List(Group20Var0, Group20Var1, Group20Var2, Group20Var5, Group20Var6)

  def group: Byte = 20

  def desc: String = "Counter"

  override def groupType: GroupType = GroupType.StaticCounter
}

object Group20Var0 extends AnyVariation(Group20, 0)

object Group20Var1 extends FixedSize(Group20, 1, bit32WithFlags)(flags, count32)

object Group20Var2 extends FixedSize(Group20, 2, bit16WithFlags)(flags, count16)

object Group20Var5 extends FixedSize(Group20, 5, bit32WithoutFlags)(count32)

object Group20Var6 extends FixedSize(Group20, 6, bit16WithoutFlags)(count16)


