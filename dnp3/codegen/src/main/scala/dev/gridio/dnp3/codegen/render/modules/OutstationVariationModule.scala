// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model.{NoTimeTargets, OutstationFamily, RustScope, Variation}
import dev.gridio.dnp3.codegen.render.*
import dev.gridio.dnp3.codegen.render.given_Conversion_String_RenderString

/**
 * One variation enum of an outstation family: its static or its event variations
 */
private case class OutstationEnum(family: OutstationFamily, name: String, variations: List[Variation], isEvent: Boolean) {

  def kind: String = if (isEvent) "event" else "static"

  def snakeName: String = OutstationEnum.snakeCase(name)

  def noTimeTargets(v: Variation): Option[NoTimeTargets] = if (isEvent) family.noTimeTargets(v) else None
}

private object OutstationEnum {

  def snakeCase(s: String): String = s.flatMap(c => if (c.isUpper) s"_${c.toLower}" else c.toString).stripPrefix("_")

  def of(family: OutstationFamily): List[OutstationEnum] = {
    OutstationEnum(family, family.staticEnumName, family.staticVariations, isEvent = false) ::
      family.eventGroup.toList.map(_ => OutstationEnum(family, family.eventEnumName, family.eventVariations, isEvent = true))
  }

  def all: List[OutstationEnum] = OutstationFamily.withEnums.flatMap(of)

  def ffi: List[OutstationEnum] = all.filter(_.family.ffi)
}

/**
 * The items in order, with a blank line between each
 */
private def separated(items: List[Iterator[String]]): Iterator[String] = {
  items.iterator.zipWithIndex.flatMap { case (item, i) => if (i == 0) item else space ++ item }
}

/**
 * The static and event variation enums of the outstation's point types
 */
object OutstationVariationModule extends Module {

  override def lines(implicit indent: Indentation): Iterator[String] = {
    separated(OutstationEnum.all.map(e => definition(e)))
  }

  private def noTimeNote(t: NoTimeTargets): List[String] = t.unsynchronized match {
    case None => List(s"Events recorded without time are reported as `${t.withoutTime.name}`")
    case Some(u) => List(
      s"Events recorded with unsynchronized time are reported as `${u.name}` after an",
      "unsynchronized common time of occurrence (g51v2), and events recorded without time as",
      s"`${t.withoutTime.name}`"
    )
  }

  private def definition(e: OutstationEnum)(implicit indent: Indentation): Iterator[String] = {
    val visibility = e.family.scope match {
      case RustScope.Public => "pub"
      case RustScope.Crate => "pub(crate)"
      case RustScope.UnitStructs => throw new Exception(s"${e.family.name} has no generated enums")
    }

    commented(s"Enum representing all possible `${e.family.name}` ${e.kind} variations").eol ++
    "#[derive(Copy, Clone, Debug, PartialEq, Eq)]".eol ++
    "#[cfg_attr(".eol ++
    indent {
      "feature = \"serialization\",".eol ++
      "derive(serde::Serialize, serde::Deserialize)".eol
    } ++
    ")]".eol ++
    bracket(s"$visibility enum ${e.name}") {
      e.variations.iterator.flatMap { v =>
        commented(v.fullDesc).eol ++
        e.noTimeTargets(v).iterator.flatMap(t => "///".eol ++ noTimeNote(t).iterator.map(commented)) ++
        s"${v.name},".eol
      }
    }
  }
}

/**
 * Defines the FFI variation enums of the outstation's point types in the bindings schema
 */
object FfiOutstationVariationSchemaModule extends Module {

  override def lines(implicit indent: Indentation): Iterator[String] = {
    "use oo_bindgen::model::{doc, BackTraced, EnumHandle, LibraryBuilder};".eol ++
    space ++
    separated(OutstationFamily.withEnums.filter(_.ffi).map(f => defineFn(f))).map(l => if (l.isBlank) "" else l)
  }

  private def link(e: OutstationEnum, v: Variation): String = s"{enum:${e.snakeName}.${FfiVariation.schemaName(v)}}"

  private def noTimeNote(e: OutstationEnum, t: NoTimeTargets): String = t.unsynchronized match {
    case None => s"Events recorded without time are reported as ${link(e, t.withoutTime)}."
    case Some(u) =>
      s"Events recorded with unsynchronized time are reported as ${link(e, u)} after an unsynchronized common time of occurrence (g51v2), and events recorded without time as ${link(e, t.withoutTime)}."
  }

  private def push(e: OutstationEnum, v: Variation)(implicit indent: Indentation): Iterator[String] = {
    val name = quoted(FfiVariation.schemaName(v))
    val desc = quoted(v.fullDesc)
    e.noTimeTargets(v) match {
      case Some(t) =>
        ".push(".eol ++ indent {
          s"$name,".eol ++
          s"doc($desc)".eol ++
          indent {
            s".details(${quoted(noTimeNote(e, t))}),".eol
          }
        } ++ ")?".eol
      case None =>
        // the line length at which rustfmt breaks the call's arguments onto their own lines
        if (s"$name, $desc".length <= 60) {
          s".push($name, $desc)?".eol
        } else {
          ".push(".eol ++ indent {
            s"$name,".eol ++ s"$desc,".eol
          } ++ ")?".eol
        }
    }
  }

  private def defineEnum(e: OutstationEnum, binding: String)(implicit indent: Indentation): Iterator[String] = {
    s"let $binding = lib".eol ++ indent {
      s".define_enum(${quoted(e.snakeName)})?".eol ++
      e.variations.iterator.flatMap(v => push(e, v)) ++
      s".doc(${quoted(s"${e.kind.capitalize} ${e.family.words} variation")})?".eol ++
      ".build()?;".eol
    }
  }

  private def defineFn(family: OutstationFamily)(implicit indent: Indentation): Iterator[String] = {
    val enums = OutstationEnum.of(family)
    val bindings = enums.map(e => s"${e.kind}_variation")
    commented(s"Defines the ${enums.map(_.kind).mkString(" and ")} variation enums of the ${family.words} points").eol ++
    s"pub(crate) fn define_${OutstationEnum.snakeCase(family.name)}_variations(".eol ++
    indent {
      "lib: &mut LibraryBuilder,".eol
    } ++
    bracket(s") -> BackTraced<(${enums.map(_ => "EnumHandle").mkString(", ")})>") {
      enums.zip(bindings).iterator.flatMap { case (e, b) => defineEnum(e, b) ++ space } ++
      s"Ok((${bindings.mkString(", ")}))".eol
    }
  }
}

/**
 * Converts the FFI variation enums of the outstation's point types to the Rust ones
 */
object FfiOutstationVariationConversionModule extends Module {

  override def lines(implicit indent: Indentation): Iterator[String] = {
    "use dnp3::outstation::database::*;".eol ++
    space ++
    "use crate::ffi;".eol ++
    space ++
    separated(OutstationEnum.ffi.map(e => conversion(e)))
  }

  private def conversion(e: OutstationEnum)(implicit indent: Indentation): Iterator[String] = {
    bracket(s"impl From<ffi::${e.name}> for ${e.name}") {
      bracket(s"fn from(from: ffi::${e.name}) -> Self") {
        bracket("match from") {
          e.variations.iterator.flatMap(v => s"ffi::${e.name}::${v.name} => Self::${v.name},".eol)
        }
      }
    }
  }
}
