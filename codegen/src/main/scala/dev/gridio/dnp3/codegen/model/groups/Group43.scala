package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._
import dev.gridio.dnp3.codegen.model.{FixedSize, GroupVariation, ObjectGroup}

//analog output events
object Group43 extends ObjectGroup {
  def variations: List[GroupVariation] = List(Group43Var1, Group43Var2, Group43Var3, Group43Var4, Group43Var5, Group43Var6, Group43Var7, Group43Var8)

  def group: Byte = 43

  def desc: String = "Analog Command Event"

  def isEventGroup: Boolean = true
}

object Group43Var1 extends FixedSize(Group43, 1, bit32)(commandStatus, value32)

object Group43Var2 extends FixedSize(Group43, 2, bit16)(commandStatus, value16)

object Group43Var3 extends FixedSize(Group43, 3, bit32WithTime)(commandStatus, value32, time48)

object Group43Var4 extends FixedSize(Group43, 4, bit16WithTime)(commandStatus, value16, time48)

object Group43Var5 extends FixedSize(Group43, 5, singlePrecision)(commandStatus, float32)

object Group43Var6 extends FixedSize(Group43, 6, doublePrecision)(commandStatus, float64)

object Group43Var7 extends FixedSize(Group43, 7, singlePrecisionWithTime)(commandStatus, float32, time48)

object Group43Var8 extends FixedSize(Group43, 8, doublePrecisionWithTime)(commandStatus, float64, time48)