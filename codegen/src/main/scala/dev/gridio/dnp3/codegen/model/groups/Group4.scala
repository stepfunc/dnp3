package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._

object Group4 extends ObjectGroup {
  def variations: List[Variation] = List(Group4Var0, Group4Var1, Group4Var2, Group4Var3)

  def group: Byte = 4

  def desc: String = "Double-bit Binary Input Event"

  override def groupType: GroupType = GroupType.DoubleBinaryEvent
}

object Group4Var0 extends AnyVariation(Group4, 0)

object Group4Var1 extends FixedSize(Group4, 1, withoutTime)(flags)

object Group4Var2 extends FixedSize(Group4, 2, withAbsoluteTime)(flags, time48)

object Group4Var3 extends FixedSize(Group4, 3, withRelativeTime)(flags, time16)