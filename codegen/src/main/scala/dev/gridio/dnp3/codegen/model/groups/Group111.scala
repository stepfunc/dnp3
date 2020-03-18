package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.{GroupVariation, ObjectGroup, SizedByVariation}

object Group111 extends ObjectGroup {
  def objects: List[GroupVariation] = List(Group111AnyVar)

  def group: Byte = 111

  def desc: String = "Octet String Event"

  def isEventGroup: Boolean = false
}

object Group111AnyVar extends SizedByVariation(Group111, 0)