package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.{FixedSize, GroupVariation, ObjectGroup}

object Group52 extends ObjectGroup {
  def objects: List[GroupVariation] = List(Group52Var1, Group52Var2)

  def group: Byte = 52

  def desc: String = "Time Delay"

  def isEventGroup: Boolean = false
}

object Group52Var1 extends FixedSize(Group52, 1, "Coarse")(time16)

object Group52Var2 extends FixedSize(Group52, 2, "Fine")(time16)