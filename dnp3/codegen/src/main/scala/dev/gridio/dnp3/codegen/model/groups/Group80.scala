// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.VariationNames._

object Group80 extends ObjectGroup {
  def variations: List[Variation] = List(Group80Var1)

  def group: Byte = 80

  def desc: String = "Internal Indications"

  override def groupType: GroupType = GroupType.InternalIndications
}

object Group80Var1 extends SingleBitField(Group80, 1, packedFormat)

