// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._

// frozen counter events
object Group23 extends ObjectGroup {
  def variations: List[Variation] = List(Group23Var0, Group23Var1, Group23Var2, Group23Var5, Group23Var6)

  def group: Byte = 23

  def desc: String = "Frozen Counter Event"

  override def groupType: GroupType = GroupType.FrozenCounterEvent
}

object Group23Var0 extends AnyVariation(Group23, 0)

object Group23Var1 extends FixedSize(Group23, 1, bit32WithFlag)(flags, count32)

object Group23Var2 extends FixedSize(Group23, 2, bit16WithFlag)(flags, count16)

object Group23Var5 extends FixedSize(Group23, 5, bit32WithFlagTime)(flags, count32, time48)

object Group23Var6 extends FixedSize(Group23, 6, bit16WithFlagTime)(flags, count16, time48)
