// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model.groups.*
import dev.gridio.dnp3.codegen.model.{ObjectGroup, SizedByVariation, Variation}
import dev.gridio.dnp3.codegen.render.*
import dev.gridio.dnp3.codegen.render.given_Conversion_String_RenderString

/**
 * The FFI `Variation` enum: one value for each variant of the Rust `Variation` enum
 */
private object FfiVariation {

  /**
   * The order of the FFI values, which sets their published discriminants. It is the model's order,
   * except that the specific attribute (`Group0(u8)`) comes before `Group0Var254`.
   */
  val all: List[Variation] = SpecificAttribute :: ObjectGroup.allVariations.filterNot(_ == SpecificAttribute)

  /**
   * Rust variants that carry the variation as a payload map to one FFI value without it
   */
  private def hasPayload(v: Variation): Boolean = v match {
    case _: SizedByVariation => true
    case SpecificAttribute => true
    case _ => false
  }

  def schemaName(v: Variation): String = {
    if (hasPayload(v)) s"group${v.group}" else s"group${v.group}_var${v.variation}"
  }

  /**
   * The variant name, the same in `dnp3::app::Variation` and `ffi::Variation`
   */
  private def variantName(v: Variation): String = {
    if (hasPayload(v)) v.parent.name else v.name
  }

  def ffiValue(v: Variation): String = s"ffi::Variation::${variantName(v)}"

  /**
   * The Rust variation an FFI value converts to: payload-carrying variants get a payload of zero
   */
  def rustValue(v: Variation): String = {
    if (hasPayload(v)) s"Variation::${variantName(v)}(0)" else s"Variation::${variantName(v)}"
  }

  def rustPattern(v: Variation): String = {
    if (hasPayload(v)) s"Variation::${variantName(v)}(_)" else s"Variation::${variantName(v)}"
  }
}

/**
 * Defines the FFI `Variation` enum in the bindings schema
 */
object FfiVariationSchemaModule extends Module {

  override def lines(implicit indent: Indentation): Iterator[String] = {
    "use oo_bindgen::model::{BackTraced, EnumHandle, LibraryBuilder};".eol ++
    space ++
    bracket("pub(crate) fn define(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle>") {
      "let variation = lib".eol ++
      indent {
        ".define_enum(\"variation\")?".eol ++
        FfiVariation.all.iterator.flatMap { v =>
          s".push(${quoted(FfiVariation.schemaName(v))}, ${quoted(v.fullDesc)})?".eol
        } ++
        ".doc(\"Group/Variation\")?".eol ++
        ".build()?;".eol
      } ++
      "Ok(variation)".eol
    }
  }
}

/**
 * Converts between the FFI `Variation` enum and `dnp3::app::Variation`
 */
object FfiVariationConversionModule extends Module {

  override def lines(implicit indent: Indentation): Iterator[String] = {
    "use dnp3::app::Variation;".eol ++
    space ++
    "use crate::ffi;".eol ++
    space ++
    bracket("impl From<ffi::Variation> for Variation") {
      bracket("fn from(from: ffi::Variation) -> Variation") {
        bracket("match from") {
          FfiVariation.all.iterator.flatMap { v =>
            s"${FfiVariation.ffiValue(v)} => ${FfiVariation.rustValue(v)},".eol
          }
        }
      }
    } ++
    space ++
    bracket("impl From<Variation> for ffi::Variation") {
      bracket("fn from(from: Variation) -> ffi::Variation") {
        bracket("match from") {
          FfiVariation.all.iterator.flatMap { v =>
            s"${FfiVariation.rustPattern(v)} => ${FfiVariation.ffiValue(v)},".eol
          }
        }
      }
    }
  }
}
