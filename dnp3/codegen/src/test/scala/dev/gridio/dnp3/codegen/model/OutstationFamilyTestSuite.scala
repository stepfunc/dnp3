// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.model

import dev.gridio.dnp3.codegen.model.FixedSizeField.*
import org.scalatest.funsuite.AnyFunSuite
import org.scalatest.matchers.should.Matchers

class OutstationFamilyTestSuite extends AnyFunSuite with Matchers {

  private object TestGroup extends ObjectGroup {
    def variations: List[Variation] = Nil
    def group: Byte = 200.toByte
    def desc: String = "Test Event"
    def groupType: GroupType = GroupType.CounterEvent
  }

  private def fixed(v: Byte, fields: FixedSizeField*): Variation = new FixedSize(TestGroup, v, "test")(fields*)

  test("the no-time target of a variation is the one with its timestamp removed") {
    val noTime = fixed(1, flags, count32)
    val absolute = fixed(2, flags, count32, time48)
    val relative = fixed(3, flags, count32, time16)
    val all = List(noTime, absolute, relative)
    OutstationFamily.noTimeTargets(absolute, all) should equal(Some(NoTimeTargets(noTime, Some(relative))))
    OutstationFamily.noTimeTargets(relative, all) should equal(Some(NoTimeTargets(noTime, None)))
    OutstationFamily.noTimeTargets(noTime, all) should equal(None)
  }

  test("a field without a timestamp attribute is not a timestamp") {
    val delay = fixed(1, flags, delay16)
    OutstationFamily.noTimeTargets(delay, List(delay)) should equal(None)
  }

  test("a missing no-time target fails") {
    val absolute = fixed(2, flags, count32, time48)
    an[Exception] should be thrownBy OutstationFamily.noTimeTargets(absolute, List(absolute))
  }

  test("an ambiguous no-time target fails") {
    val absolute = fixed(2, flags, count32, time48)
    val all = List(fixed(1, flags, count32), fixed(3, flags, count32), absolute)
    an[Exception] should be thrownBy OutstationFamily.noTimeTargets(absolute, all)
  }

  test("an ambiguous unsynchronized target fails") {
    val absolute = fixed(2, flags, count32, time48)
    val all = List(fixed(1, flags, count32), absolute, fixed(3, flags, count32, time16), fixed(4, flags, count32, time16))
    an[Exception] should be thrownBy OutstationFamily.noTimeTargets(absolute, all)
  }

  test("a variation with two timestamps fails") {
    val both = fixed(2, flags, time48, time16)
    an[Exception] should be thrownBy OutstationFamily.noTimeTargets(both, List(fixed(1, flags, time16), both))
  }
}
