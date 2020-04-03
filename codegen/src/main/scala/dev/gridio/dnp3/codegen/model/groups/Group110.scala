package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._

object Group110 extends ObjectGroup {
  def variations: List[Variation] = List(Group110AnyVar)

  def group: Byte = 110

  def desc: String = "Octet String"

  override def groupType: GroupType = GroupType.StaticOctetString
}

object Group110AnyVar extends SizedByVariation(Group110, 0)
