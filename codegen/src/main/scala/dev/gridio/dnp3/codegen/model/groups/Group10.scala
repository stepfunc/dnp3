package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._
import dev.gridio.dnp3.codegen.model._

object Group10 extends ObjectGroup {
  def variations: List[GroupVariation] = List(Group10Var0, Group10Var1, Group10Var2)

  def desc: String = "Binary Output"

  def group: Byte = 10

  def isEventGroup: Boolean = false
}

object Group10Var0 extends AnyVariation(Group10, 0)

object Group10Var1 extends SingleBitfield(Group10, 1, packedFormat)

object Group10Var2 extends FixedSize(Group10, 2, outputStatusWithFlags)(flags) with StaticVariation.BinaryOutputStatus

