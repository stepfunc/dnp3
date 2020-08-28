package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._

object Group113 extends ObjectGroup {
  def variations: List[Variation] = List(Group113AnyVar)

  def group: Byte = 113

  def desc: String = "Virtual Terminal Event Data"

  override def groupType: GroupType = GroupType.VirtualTerminalEvent
}

object Group113AnyVar extends SizedByVariation(Group113, 0)