// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._
import dev.gridio.dnp3.codegen.model._

// frozen counters
object Group21 extends ObjectGroup {

  def variations: List[Variation] = List(
    Group21Var0,
    Group21Var1,
    Group21Var2,
    Group21Var5,
    Group21Var6,
    Group21Var9,
    Group21Var10
  )

  def group: Byte = 21

  def desc: String = "Frozen Counter"

  override def groupType: GroupType = GroupType.StaticFrozenCounter

}

object Group21Var0 extends AnyVariation(Group21, 0)

object Group21Var1 extends FixedSize(Group21, 1, bit32WithFlags)(flags, count32)

object Group21Var2 extends FixedSize(Group21, 2, bit16WithFlags)(flags, count16)

object Group21Var5 extends FixedSize(Group21, 5, bit32WithFlagsAndTimeOfFreeze)(flags, count32, time48)

object Group21Var6 extends FixedSize(Group21, 6, bit16WithFlagsAndTimeOfFreeze)(flags, count16, time48)

object Group21Var9 extends FixedSize(Group21, 9, bit32WithoutFlags)(count32)

object Group21Var10 extends FixedSize(Group21, 10, bit16WithoutFlags)(count16)
