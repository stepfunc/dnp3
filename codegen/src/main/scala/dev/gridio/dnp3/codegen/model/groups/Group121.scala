package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model._


object Group121 extends ObjectGroup {
  def variations: List[GroupVariation] = List(Group121Var0, Group121Var1)

  def group: Byte = 121

  def desc: String = "Security statistic"

  def isEventGroup: Boolean = false
}

object Group121Var0 extends AnyVariation(Group121, 0)

object Group121Var1 extends FixedSize(Group121, 1, VariationNames.bit32WithFlag)(flags, assocId, count32)

