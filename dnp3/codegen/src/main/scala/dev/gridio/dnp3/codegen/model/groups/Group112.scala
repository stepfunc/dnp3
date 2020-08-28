package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._

object Group112 extends ObjectGroup {
  def variations: List[Variation] = List(Group112AnyVar)

  def group: Byte = 112

  def desc: String = "Virtual Terminal Output Block"

  override def groupType: GroupType = GroupType.VirtualTerminalOutput
}

object Group112AnyVar extends SizedByVariation(Group112, 0)