package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._

object Group102 extends ObjectGroup {
  def variations: List[Variation] = List(Group102Var0, Group102Var1)

  def group: Byte = 102

  def desc: String = "Unsigned Integer"

  override def groupType: GroupType = GroupType.StaticUnsignedInteger
}

object Group102Var0 extends AnyVariation(Group102, 0)

object Group102Var1 extends FixedSize(Group102, 1, "8-bit")(unsignedByte)

