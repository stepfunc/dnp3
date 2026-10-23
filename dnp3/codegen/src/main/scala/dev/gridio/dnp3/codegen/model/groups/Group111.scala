// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._

object Group111 extends ObjectGroup {
  def variations: List[Variation] = List(Group111AnyVar)

  def group: Byte = 111

  def desc: String = "Octet String Event"

  override def groupType: GroupType = GroupType.OctetStringEvent
}

object Group111AnyVar extends SizedByVariation(Group111, 0)