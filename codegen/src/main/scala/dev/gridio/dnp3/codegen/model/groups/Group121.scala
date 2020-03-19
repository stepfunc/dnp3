package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model._


object Group121 extends ObjectGroup {
  def variations: List[Variation] = List(Group121Var0, Group121Var1)

  def group: Byte = 121

  def desc: String = "Security statistic"

  override def groupType: GroupType = OtherGroupType
}

object Group121Var0 extends AnyVariation(Group121, 0)

object Group121Var1 extends FixedSize(Group121, 1, VariationNames.bit32WithFlag)(flags, assocId, count32)

