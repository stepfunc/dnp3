package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.FixedSizeField._

// absolute time
object Group50 extends ObjectGroup {
  def variations: List[Variation] = List(Group50Var1, Group50Var2, Group50Var3, Group50Var4)

  def group: Byte = 50

  def desc: String = "Time and Date"

  override def groupType: GroupType = GroupType.Time
}

object Group50Var1 extends FixedSize(Group50, 1, "Absolute Time")(time48)

object Group50Var2 extends FixedSize(Group50, 2, "Absolute time and interval")(
  time48,
  FixedSizeField("interval", UInt32Field)
)

object Group50Var3 extends FixedSize(Group50, 3, "Absolute Time at last recorded time")(time48)

object Group50Var4 extends FixedSize(Group50, 4, "Indexed absolute time and long interval")(
  time48,
  FixedSizeField("interval", UInt32Field),
  FixedSizeField("units", UInt8Field)
)

