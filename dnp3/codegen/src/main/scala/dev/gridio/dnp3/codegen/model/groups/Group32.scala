// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._

// analog input event
object Group32 extends ObjectGroup {
  def variations: List[Variation] = List(Group32Var0, Group32Var1, Group32Var2, Group32Var3, Group32Var4, Group32Var5, Group32Var6, Group32Var7, Group32Var8)

  def group: Byte = 32

  def desc: String = "Analog Input Event"

  override def groupType: GroupType = GroupType.AnalogEvent
}

object Group32Var0 extends AnyVariation(Group32, 0)

object Group32Var1 extends FixedSize(Group32, 1, bit32WithoutTime)(flags, value32)

object Group32Var2 extends FixedSize(Group32, 2, bit16WithoutTime)(flags, value16)

object Group32Var3 extends FixedSize(Group32, 3, bit32WithTime)(flags, value32, time48)

object Group32Var4 extends FixedSize(Group32, 4, bit16WithTime)(flags, value16, time48)

object Group32Var5 extends FixedSize(Group32, 5, singlePrecisionWithoutTime)(flags, float32)

object Group32Var6 extends FixedSize(Group32, 6, doublePrecisionWithoutTime)(flags, float64)

object Group32Var7 extends FixedSize(Group32, 7, singlePrecisionWithTime)(flags, float32, time48)

object Group32Var8 extends FixedSize(Group32, 8, doublePrecisionWithTime)(flags, float64, time48)
