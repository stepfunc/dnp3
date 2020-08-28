package dev.gridio.dnp3.codegen.model.groups

import dev.gridio.dnp3.codegen.model._

object Group60 extends ObjectGroup {
  def variations: List[Variation] = List(Group60Var1, Group60Var2, Group60Var3, Group60Var4)

  def group: Byte = 60

  def desc: String = "Class Data"

  override def groupType: GroupType = GroupType.ClassData
}

object Group60Var1 extends ClassData(Group60, 1, "Class 0")

object Group60Var2 extends ClassData(Group60, 2, "Class 1")

object Group60Var3 extends ClassData(Group60, 3, "Class 2")

object Group60Var4 extends ClassData(Group60, 4, "Class 3")
