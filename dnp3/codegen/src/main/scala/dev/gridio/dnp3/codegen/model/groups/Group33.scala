package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._

object Group33 extends ObjectGroup {
  def variations: List[Variation] = List(Group33Var0, Group33Var1, Group33Var2, Group33Var3, Group33Var4, Group33Var5, Group33Var6, Group33Var7, Group33Var8)

  def group: Byte = 33

  def desc: String = "Frozen Analog Input Event"

  override def groupType: GroupType = GroupType.FrozenAnalogEvent
}

object Group33Var0 extends AnyVariation(Group33, 0)

object Group33Var1 extends FixedSize(Group33, 1, bit32WithFlag)(flags, value32)
object Group33Var2 extends FixedSize(Group33, 2, bit16WithFlag)(flags, value16)
object Group33Var3 extends FixedSize(Group33, 3, bit32WithFlagAndTimeOfFreeze)(flags, value32, time48)
object Group33Var4 extends FixedSize(Group33, 4, bit16WithFlagAndTimeOfFreeze)(flags, value16, time48)

object Group33Var5 extends FixedSize(Group33, 5, singlePrecisionWithFlag)(flags, float32)
object Group33Var6 extends FixedSize(Group33, 6, doublePrecisionWithFlag)(flags, float64)
object Group33Var7 extends FixedSize(Group33, 7, singlePrecisionWithFlagTime)(flags, float32, time48)
object Group33Var8 extends FixedSize(Group33, 8, doublePrecisionWithFlagTime)(flags, float64, time48)
