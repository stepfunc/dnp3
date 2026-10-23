// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.FixedSizeField._

object Group52 extends ObjectGroup {
  def variations: List[Variation] = List(Group52Var1, Group52Var2)

  def group: Byte = 52

  def desc: String = "Time Delay"

  override def groupType: GroupType = GroupType.Time
}

object Group52Var1 extends FixedSize(Group52, 1, "Coarse")(delay16)

object Group52Var2 extends FixedSize(Group52, 2, "Fine")(delay16)