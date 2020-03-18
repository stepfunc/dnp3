package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._
import dev.gridio.dnp3.codegen.model.{FixedSize, GroupVariation, ObjectGroup}


object Group13 extends ObjectGroup {
  def objects: List[GroupVariation] = List(Group13Var1, Group13Var2)

  def group: Byte = 13

  def desc: String = "Binary Command Event"

  def isEventGroup: Boolean = true
}

object Group13Var1 extends FixedSize(Group13, 1, withoutTime)(flags)

object Group13Var2 extends FixedSize(Group13, 2, withTime)(flags, time48)