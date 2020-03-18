package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.VariationNames._
import dev.gridio.dnp3.codegen.model.{GroupVariation, ObjectGroup, SingleBitfield}

object Group80 extends ObjectGroup {
  def objects: List[GroupVariation] = List(Group80Var1)

  def group: Byte = 80

  def desc: String = "Internal Indications"

  def isEventGroup: Boolean = false
}

object Group80Var1 extends SingleBitfield(Group80, 1, packedFormat)

