package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._

object Group0 extends ObjectGroup {

  def variations: List[Variation] = List(AllAttributesRequest, SpecificAttribute)

  def desc: String = "Device Attributes"

  def group: Byte = 0

  override def groupType: GroupType = GroupType.DeviceAttributes
}

object SpecificAttribute extends BasicGroupVariation (Group0,  Variation.Value(0), "Specific Attribute")
object AllAttributesRequest extends BasicGroupVariation (Group0,  Variation.Value(254), "Non-Specific All Attributes Request")



