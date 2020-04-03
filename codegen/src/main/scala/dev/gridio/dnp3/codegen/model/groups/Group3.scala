package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._

object Group3 extends ObjectGroup {
  def variations: List[Variation] = List(Group3Var0, Group3Var1, Group3Var2)

  def group: Byte = 3

  def desc: String = "Double-bit Binary Input"

  override def groupType: GroupType = GroupType.StaticDoubleBinary
}

object Group3Var0 extends AnyVariation(Group3, 0)

object Group3Var1 extends DoubleBitField(Group3, 1, packedFormat)

object Group3Var2 extends FixedSize(Group3, 2, withFlags)(flags)
