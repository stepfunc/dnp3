package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._

object Group31 extends ObjectGroup {
  def variations: List[Variation] = List(Group31Var0, Group31Var1, Group31Var2, Group31Var3, Group31Var4, Group31Var5, Group31Var6, Group31Var7, Group31Var8)

  def group: Byte = 31

  def desc: String = "Frozen Analog Input"

  override def groupType: GroupType = GroupType.StaticFrozenAnalog
}

object Group31Var0 extends AnyVariation(Group31, 0)

object Group31Var1 extends FixedSize(Group31, 1, bit32WithFlag)(flags, value32)
object Group31Var2 extends FixedSize(Group31, 2, bit16WithFlag)(flags, value16)
object Group31Var3 extends FixedSize(Group31, 3, bit32WithFlagAndTimeOfFreeze)(flags, value32, time48)
object Group31Var4 extends FixedSize(Group31, 4, bit16WithFlagAndTimeOfFreeze)(flags, value16, time48)

object Group31Var5 extends FixedSize(Group31, 5, bit32WithoutFlag)(value32)
object Group31Var6 extends FixedSize(Group31, 6, bit16WithoutFlag)(value16)
object Group31Var7 extends FixedSize(Group31, 7, singlePrecisionWithFlag)(flags, float32)
object Group31Var8 extends FixedSize(Group31, 8, doublePrecisionWithFlag)(flags, float64)
