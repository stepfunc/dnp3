// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model.*
import dev.gridio.dnp3.codegen.model.groups.SpecificAttribute
import dev.gridio.dnp3.codegen.render.*
import dev.gridio.dnp3.codegen.render.given_Conversion_String_RenderString

/**
 * What the outstation selects when a READ names each variation: the static points or events of a
 * family, a class, or a set of device attributes
 */
object OutstationReadTargetModule extends Module {

  private case class Arm(pattern: String, target: String)

  override def lines(implicit indent: Indentation): Iterator[String] = {
    "use crate::app::Variation;".eol ++
    "use crate::outstation::database::variations::*;".eol ++
    "use crate::outstation::database::EventClass;".eol ++
    space ++
    "use super::{EventTarget, ReadTarget, StaticTarget};".eol ++
    space ++
    commented("What a READ of a variation selects, whatever its qualifier, or `None` if a READ can't name it").eol ++
    bracket("pub(super) fn get(variation: Variation) -> Option<ReadTarget>") {
      uses.iterator.map(u => s"use $u;") ++
      space ++
      bracketSemi("let target = match variation") {
        readable.iterator.flatMap { case (g, arms) =>
          s"// group ${g.group}".eol ++ arms.iterator.map(a => s"${a.pattern} => ${a.target},")
        } ++
        "// not readable".eol ++
        notReadableArm
      } ++
      "Some(target)".eol
    }
  }.map(l => if (l.isBlank) "" else l)

  private def notReadableArm: Iterator[String] = {
    val patterns = notReadable
    patterns.iterator.zipWithIndex.map { case (p, i) =>
      val line = if (i == 0) p else s"| $p"
      if (i == patterns.size - 1) s"$line => return None," else line
    }
  }

  private def bracketSemi(s: String)(inner: => Iterator[String])(implicit indent: Indentation): Iterator[String] = {
    (s + " {").eol ++ indent(inner) ++ "};".eol
  }

  // in the order rustfmt sorts them
  private def uses: List[String] = {
    val enums = OutstationEnum.all.map(e => s"${e.name}::*")
    ("ReadTarget::{Attr, Class0, Event, Static}" :: enums).sorted
  }

  private def familyArms(family: OutstationFamily, g: ObjectGroup, isEvent: Boolean): List[Arm] = {
    val target = if (isEvent) "Event(EventTarget" else "Static(StaticTarget"
    family.scope match {
      case RustScope.UnitStructs => g.variations match {
        case List(_: SizedByVariation) => List(Arm(s"Variation::${g.name}(0)", s"$target::${family.readTarget})"))
        case _ => throw new Exception(s"${g.name} has variations other than one sized by variation")
      }
      case RustScope.Public | RustScope.Crate => g.variations.map {
        case v: AnyVariation => Arm(s"Variation::${v.name}", s"$target::${family.readTarget}(None))")
        case v: SizedByVariation => throw new Exception(s"${v.name} is sized by variation in a family with enums")
        case v => Arm(s"Variation::${v.name}", s"$target::${family.readTarget}(Some(${v.name})))")
      }
    }
  }

  private def attributeArms(g: ObjectGroup): List[Arm] = g.variations.map {
    case SpecificAttribute => Arm(s"Variation::${g.name}(var)", "Attr(var)")
    case v => Arm(s"Variation::${v.name}", s"Attr(${v.variation})")
  }

  // class 0 selects the static points, and classes 1 to 3 the events of that class
  private def classArms(g: ObjectGroup): List[Arm] = g.variations.map { v =>
    v.variation.value match {
      case 1 => Arm(s"Variation::${v.name}", "Class0")
      case n @ (2 | 3 | 4) => Arm(s"Variation::${v.name}", s"Event(EventTarget::Class(EventClass::Class${n - 1}))")
      case _ => throw new Exception(s"${v.name} is not a class")
    }
  }

  private def arms(g: ObjectGroup): List[Arm] = {
    val static = OutstationFamily.all.filter(_.staticGroup == g).map(f => familyArms(f, g, isEvent = false))
    val event = OutstationFamily.all.filter(_.eventGroup.contains(g)).map(f => familyArms(f, g, isEvent = true))
    val rules = g.groupType match {
      case GroupType.DeviceAttributes => List(attributeArms(g))
      case GroupType.ClassData => List(classArms(g))
      case _ => Nil
    }
    (static ++ event ++ rules) match {
      case Nil => Nil
      case List(arms) => arms
      case _ => throw new Exception(s"${g.name} has more than one READ target rule")
    }
  }

  private def readable: List[(ObjectGroup, List[Arm])] = ObjectGroup.all.map(g => (g, arms(g))).filter(_._2.nonEmpty)

  // every variation without a target, and the sizes of the sized-by-variation groups other than 0
  private def notReadable: List[String] = ObjectGroup.all.flatMap { g =>
    val handled = arms(g).nonEmpty
    g.variations.flatMap {
      case _: SizedByVariation => List(s"Variation::${g.name}(_)")
      case SpecificAttribute if !handled => List(s"Variation::${g.name}(_)")
      case v if !handled => List(s"Variation::${v.name}")
      case _ => Nil
    }
  }
}
