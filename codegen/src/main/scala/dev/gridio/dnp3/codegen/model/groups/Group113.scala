package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.{GroupVariation, ObjectGroup, SizedByVariation}

object Group113 extends ObjectGroup {
  def objects: List[GroupVariation] = List(Group113AnyVar)

  def group: Byte = 113

  def desc: String = "Virtual Terminal Event Data"

  def isEventGroup: Boolean = true
}

object Group113AnyVar extends SizedByVariation(Group113, 0)