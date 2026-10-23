// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._

object Group70 extends ObjectGroup {
  def variations: List[Variation] = List(
    Group70Var2,
    Group70Var3,
    Group70Var4,
    Group70Var5,
    Group70Var6,
    Group70Var7,
    Group70Var8,
  )

  def group: Byte = 70

  def desc: String = "File Control"

  override def groupType: GroupType = GroupType.FileControl
}

object Group70Var2 extends FreeFormat(Group70, 2, "Authentication")
object Group70Var3 extends FreeFormat(Group70, 3, "File command")
object Group70Var4 extends FreeFormat(Group70, 4, "File command status")
object Group70Var5 extends FreeFormat(Group70, 5, "File transport")
object Group70Var6 extends FreeFormat(Group70, 6, "File transport status")
object Group70Var7 extends FreeFormat(Group70, 7, "File descriptor")
object Group70Var8 extends FreeFormat(Group70, 8, "File specification string")
