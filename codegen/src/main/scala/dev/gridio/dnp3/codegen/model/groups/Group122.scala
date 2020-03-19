package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model._

object Group122 extends ObjectGroup {
  def variations: List[GroupVariation] = List(Group122Var0, Group122Var1, Group122Var2)

  def group: Byte = 122

  def desc: String = "Security Statistic event"

  def isEventGroup: Boolean = true
}

object Group122Var0 extends AnyVariation(Group122, 0)

object Group122Var1 extends FixedSize(Group122, 1, VariationNames.bit32WithFlag)(flags, assocId, count32)

object Group122Var2 extends FixedSize(Group122, 2, VariationNames.bit32WithFlagTime)(flags, assocId, count32, time48)
