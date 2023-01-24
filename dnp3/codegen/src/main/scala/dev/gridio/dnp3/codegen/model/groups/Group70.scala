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

  def desc: String = "File-control"

  override def groupType: GroupType = GroupType.FileControl
}

object Group70Var2 extends FreeFormat(Group70, 2, "authentication")
object Group70Var3 extends FreeFormat(Group70, 3, "file command")
object Group70Var4 extends FreeFormat(Group70, 4, "file command status")
object Group70Var5 extends FreeFormat(Group70, 5, "file transport")
object Group70Var6 extends FreeFormat(Group70, 6, "file transport status")
object Group70Var7 extends FreeFormat(Group70, 7, "file descriptor")
object Group70Var8 extends FreeFormat(Group70, 8, "file specification string")
