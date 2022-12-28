package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._

object Group0 extends ObjectGroup {

  def variations: List[Variation] = List(Group0VarX)

  def desc: String = "Device Attributes"

  def group: Byte = 0

  override def groupType: GroupType = GroupType.DeviceAttributes
}

object Group0VarX extends AnyDeviceAttribute(Group0)



