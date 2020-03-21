package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.VariationNames._

object Group80 extends ObjectGroup {
  def variations: List[Variation] = List(Group80Var1)

  def group: Byte = 80

  def desc: String = "Internal Indications"

  override def groupType: GroupType = GroupType.InternalIndications
}

object Group80Var1 extends SingleBitField(Group80, 1, packedFormat)

