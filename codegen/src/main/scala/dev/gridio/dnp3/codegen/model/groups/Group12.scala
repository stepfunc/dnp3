package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.VariationNames._
import dev.gridio.dnp3.codegen.model._

object Group12 extends ObjectGroup {
  def variations: List[Variation] = List(Group12Var0, Group12Var1)

  def group: Byte = 12

  def desc: String = "Binary Command"

  override def groupType: GroupType = OtherGroupType
}

object Group12Var0 extends AnyVariation(Group12, 0)

object Group12Var1 extends FixedSize(Group12, 1, crob)(
  FixedSizeField("code", UInt8Field),
  FixedSizeField("count", UInt8Field),
  FixedSizeField("on_time", UInt32Field),
  FixedSizeField("off_time", UInt32Field),
  commandStatus
)
