package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.{EventGroupType, GroupType, ObjectGroup, SizedByVariation, Variation}

object Group111 extends ObjectGroup {
  def variations: List[Variation] = List(Group111AnyVar)

  def group: Byte = 111

  def desc: String = "Octet String Event"

  override def groupType: GroupType = EventGroupType
}

object Group111AnyVar extends SizedByVariation(Group111, 0)