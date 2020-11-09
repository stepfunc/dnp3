package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._

object Group13 extends ObjectGroup {
  def variations: List[Variation] = List(Group13Var1, Group13Var2)

  def group: Byte = 13

  def desc: String = "Binary Output Command Event"

  override def groupType: GroupType = GroupType.BinaryOutputCommandEvent
}

object Group13Var1 extends FixedSize(Group13, 1, withoutTime)(flags)

object Group13Var2 extends FixedSize(Group13, 2, withTime)(flags, time48)