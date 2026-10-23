// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._

object Group60 extends ObjectGroup {
  def variations: List[Variation] = List(Group60Var1, Group60Var2, Group60Var3, Group60Var4)

  def group: Byte = 60

  def desc: String = "Class Data"

  override def groupType: GroupType = GroupType.ClassData
}

object Group60Var1 extends ClassData(Group60, 1, "Class 0")

object Group60Var2 extends ClassData(Group60, 2, "Class 1")

object Group60Var3 extends ClassData(Group60, 3, "Class 2")

object Group60Var4 extends ClassData(Group60, 4, "Class 3")
