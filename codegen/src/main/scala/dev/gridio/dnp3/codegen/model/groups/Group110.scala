package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.{GroupVariation, ObjectGroup, SizedByVariation}

object Group110 extends ObjectGroup {
  def objects: List[GroupVariation] = List(Group110AnyVar)

  def group: Byte = 110

  def desc: String = "Octet String"

  def isEventGroup: Boolean = false
}

object Group110AnyVar extends SizedByVariation(Group110, 0)
