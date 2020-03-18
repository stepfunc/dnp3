package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model.FixedSizeField._
import dev.gridio.dnp3.codegen.model.{FixedSize, GroupVariation, ObjectGroup}

// common time of occurrence
object Group51 extends ObjectGroup {
  def objects: List[GroupVariation] = List(Group51Var1, Group51Var2)

  def group: Byte = 51

  def desc: String = "Time and Date CTO"

  def isEventGroup: Boolean = false
}

object Group51Var1 extends FixedSize(Group51, 1, "Absolute time, synchronized")(time48)

object Group51Var2 extends FixedSize(Group51, 2, "Absolute time, unsynchronized")(time48)
