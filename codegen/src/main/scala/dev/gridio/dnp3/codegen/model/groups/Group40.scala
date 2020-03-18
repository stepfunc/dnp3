package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._
import dev.gridio.dnp3.codegen.model._

// Analog output status
object Group40 extends ObjectGroup {
  def objects: List[GroupVariation] = List(Group40Var0, Group40Var1, Group40Var2, Group40Var3, Group40Var4)

  def group: Byte = 40

  def desc: String = "Analog Output Status"

  def isEventGroup: Boolean = false
}

object Group40Var0 extends AnyVariation(Group40, 0)

object Group40Var1 extends FixedSize(Group40, 1, bit32WithFlag)(flags, value32) with StaticVariation.AnalogOutputStatus

object Group40Var2 extends FixedSize(Group40, 2, bit16WithFlag)(flags, value16) with StaticVariation.AnalogOutputStatus

object Group40Var3 extends FixedSize(Group40, 3, singlePrecisionWithFlag)(flags, float32) with StaticVariation.AnalogOutputStatus

object Group40Var4 extends FixedSize(Group40, 4, doublePrecisionWithFlag)(flags, float64) with StaticVariation.AnalogOutputStatus
