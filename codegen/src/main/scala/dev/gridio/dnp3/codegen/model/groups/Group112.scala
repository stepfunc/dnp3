package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.{GroupVariation, ObjectGroup, SizedByVariation}

object Group112 extends ObjectGroup {
  def objects: List[GroupVariation] = List(Group112AnyVar)

  def group: Byte = 112

  def desc: String = "Virtual Terminal Output Block"

  def isEventGroup: Boolean = false
}

object Group112AnyVar extends SizedByVariation(Group112, 0)