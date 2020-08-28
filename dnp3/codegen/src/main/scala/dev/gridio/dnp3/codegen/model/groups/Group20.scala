package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._
import dev.gridio.dnp3.codegen.model._

// counters
object Group20 extends ObjectGroup {
  def variations: List[Variation] = List(Group20Var0, Group20Var1, Group20Var2, Group20Var5, Group20Var6)

  def group: Byte = 20

  def desc: String = "Counter"

  override def groupType: GroupType = GroupType.StaticCounter
}

object Group20Var0 extends AnyVariation(Group20, 0)

object Group20Var1 extends FixedSize(Group20, 1, bit32WithFlag)(flags, count32)

object Group20Var2 extends FixedSize(Group20, 2, bit16WithFlag)(flags, count16)

object Group20Var5 extends FixedSize(Group20, 5, bit32WithoutFlag)(count32)

object Group20Var6 extends FixedSize(Group20, 6, bit16WithoutFlag)(count16)


