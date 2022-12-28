package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._

object Group0 extends ObjectGroup {

  def variations: List[Variation] = List(ListOfAttributes, AllAttributesRequest, SpecificAttribute)

  def desc: String = "Device Attributes"

  def group: Byte = 0

  override def groupType: GroupType = GroupType.DeviceAttributes
}

object SpecificAttribute extends BasicGroupVariation (Group0,  Variation.Value(0), "Variations 1 to 253")
object AllAttributesRequest extends BasicGroupVariation (Group0,  Variation.Value(254), "Non-Specific All Attributes Request")
object ListOfAttributes extends BasicGroupVariation (Group0,  Variation.Value(255), "List of Attribute Variations")



