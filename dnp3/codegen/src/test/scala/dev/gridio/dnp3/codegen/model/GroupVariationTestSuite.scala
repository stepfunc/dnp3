package dev.gridio.dnp3.codegen.model

import dev.gridio.dnp3.codegen.model.groups.Group12Var1
import org.scalatest.funsuite.AnyFunSuite
import org.scalatest.matchers.should.Matchers

class GroupVariationTestSuite extends AnyFunSuite with Matchers {

  test("Groups are all unique") {
    ObjectGroup.all.foldLeft(Set.empty[Byte]) { (set, group) =>
      if (set(group.group)) fail("collision at " + group.group)
      set + group.group
    }
  }

  test("Object ids are not repeated") {

    val ids = for {
      group <- ObjectGroup.all
      gv <- group.variations
    } yield gv.id

    ids.foldLeft(Set.empty[Variation.Id]) { (set, id) =>
      if (set(id)) fail("collision at " + id)
      set + id
    }
  }

  test("Fixed size calculated correctly") {
    Group12Var1.size should equal(11)
  }

}
