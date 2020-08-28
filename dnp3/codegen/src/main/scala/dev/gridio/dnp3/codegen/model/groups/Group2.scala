package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._

object Group2 extends ObjectGroup {
  def variations: List[Variation] = List(Group2Var0, Group2Var1, Group2Var2, Group2Var3)

  def group: Byte = 2

  def desc: String = "Binary Input Event"

  override def groupType: GroupType = GroupType.BinaryEvent
}

object Group2Var0 extends AnyVariation(Group2, 0)

object Group2Var1 extends FixedSize(Group2, 1, withoutTime)(flags)

object Group2Var2 extends FixedSize(Group2, 2, withAbsoluteTime)(flags, time48)

object Group2Var3 extends FixedSize(Group2, 3, withRelativeTime)(flags, time16)