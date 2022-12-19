package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._

object Group34 extends ObjectGroup {
  def variations: List[Variation] = List(Group34Var0, Group34Var1, Group34Var2, Group34Var3)

  def group: Byte = 34

  def desc: String = "Analog Input Reporting Deadband"

  override def groupType: GroupType = GroupType.AnalogInputDeadband
}

object Group34Var0 extends AnyVariation(Group34, 0)

object Group34Var1 extends FixedSize(Group34, 1, bit16)(valueU16)
object Group34Var2 extends FixedSize(Group34, 2, bit32)(valueU32)
object Group34Var3 extends FixedSize(Group34, 3, singlePrecision)(float32)
