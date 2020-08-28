package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.FixedSizeField._

// common time of occurrence
object Group51 extends ObjectGroup {
  def variations: List[Variation] = List(Group51Var1, Group51Var2)

  def group: Byte = 51

  def desc: String = "Time and Date CTO"

  override def groupType: GroupType = GroupType.Time
}

object Group51Var1 extends FixedSize(Group51, 1, "Absolute time, synchronized")(time48)

object Group51Var2 extends FixedSize(Group51, 2, "Absolute time, unsynchronized")(time48)
