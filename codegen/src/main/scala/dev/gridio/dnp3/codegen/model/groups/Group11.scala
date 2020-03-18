package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._
import dev.gridio.dnp3.codegen.model.{AnyVariation, FixedSize, GroupVariation, ObjectGroup}

object Group11 extends ObjectGroup {
  def objects: List[GroupVariation] = List(Group11Var0, Group11Var1, Group11Var2)

  def group: Byte = 11

  def desc: String = "Binary Output Event"

  def isEventGroup: Boolean = true
}

object Group11Var0 extends AnyVariation(Group11, 0)

object Group11Var1 extends FixedSize(Group11, 1, outputStatusWithoutTime)(flags)

object Group11Var2 extends FixedSize(Group11, 2, outputStatusWithTime)(flags, time48)
