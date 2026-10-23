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

  /**
   * The published binding docs, where they differ from the model's description
   */
  private val descriptions: Map[Variation, String] = Map(
    SpecificAttribute -> "Device Attributes - Variations 0 to 253 and 255",
    Group1Var0 -> "Binary Input - Default variation",
    Group1Var1 -> "Binary Input - Packed format",
    Group1Var2 -> "Binary Input - With flags",
    Group2Var0 -> "Binary Input Event - Default variation",
    Group2Var1 -> "Binary Input Event - Without time",
    Group2Var2 -> "Binary Input Event - With absolute time",
    Group2Var3 -> "Binary Input Event - With relative time",
    Group3Var0 -> "Double-bit Binary Input - Default variation",
    Group3Var1 -> "Double-bit Binary Input - Packed format",
    Group3Var2 -> "Double-bit Binary Input - With flags",
    Group4Var0 -> "Double-bit Binary Input Event - Default variation",
    Group4Var1 -> "Double-bit Binary Input Event - Without time",
    Group4Var2 -> "Double-bit Binary Input Event - With absolute time",
    Group4Var3 -> "Double-bit Binary Input Event - With relative time",
    Group10Var0 -> "Binary Output - Default variation",
    Group10Var1 -> "Binary Output - Packed format",
    Group10Var2 -> "Binary Output - With flags",
    Group11Var0 -> "Binary Output Event - Default variation",
    Group11Var1 -> "Binary Output Event - Without time",
    Group11Var2 -> "Binary Output Event - With time",
    Group12Var1 -> "Binary Output Command - Control Relay Output Block",
    Group13Var1 -> "Binary Output Command Event - command status without time",
    Group13Var2 -> "Binary Output Command Event - command status with time",
    Group20Var0 -> "Counter - Default variation",
    Group20Var1 -> "Counter - 32-bit with flags",
    Group20Var2 -> "Counter - 16-bit with flags",
    Group20Var5 -> "Counter - 32-bit without flag",
    Group20Var6 -> "Counter - 16-bit without flag",
    Group21Var0 -> "Frozen Counter - Default variation",
    Group21Var1 -> "Frozen Counter - 32-bit with flags",
    Group21Var2 -> "Frozen Counter - 16-bit with flags",
    Group21Var5 -> "Frozen Counter - 32-bit with flags and time",
    Group21Var6 -> "Frozen Counter - 16-bit with flags and time",
    Group21Var9 -> "Frozen Counter - 32-bit without flag",
    Group21Var10 -> "Frozen Counter - 16-bit without flag",
    Group22Var0 -> "Counter Event - Default variation",
    Group22Var1 -> "Counter Event - 32-bit with flags",
    Group22Var2 -> "Counter Event - 16-bit with flags",
    Group22Var5 -> "Counter Event - 32-bit with flags and time",
    Group22Var6 -> "Counter Event - 16-bit with flags and time",
    Group23Var0 -> "Frozen Counter Event - Default variation",
    Group23Var1 -> "Frozen Counter Event - 32-bit with flags",
    Group23Var2 -> "Frozen Counter Event - 16-bit with flags",
    Group23Var5 -> "Frozen Counter Event - 32-bit with flags and time",
    Group23Var6 -> "Frozen Counter Event - 16-bit with flags and time",
    Group30Var0 -> "Analog Input - Default variation",
    Group30Var1 -> "Analog Input - 32-bit with flags",
    Group30Var2 -> "Analog Input - 16-bit with flags",
    Group30Var3 -> "Analog Input - 32-bit without flag",
    Group30Var4 -> "Analog Input - 16-bit without flag",
    Group30Var5 -> "Analog Input - Single-precision floating point with flags",
    Group30Var6 -> "Analog Input - Double-precision floating point with flags",
    Group31Var0 -> "Frozen Analog Input - Default variation",
    Group31Var1 -> "Frozen Analog Input - 32-bit with flags",
    Group31Var2 -> "Frozen Analog Input - 16-bit with flags",
    Group31Var3 -> "Frozen Analog Input - 32-bit with flags and time-of-freeze",
    Group31Var4 -> "Frozen Analog Input - 16-bit with flags and time-of-freeze",
    Group31Var5 -> "Frozen Analog Input - 32-bit without flags",
    Group31Var6 -> "Frozen Analog Input - 16-bit without flags",
    Group31Var7 -> "Frozen Analog Input - Single-precision floating point with flags",
    Group31Var8 -> "Frozen Analog Input - Double-precision floating point with flags",
    Group32Var0 -> "Analog Input Event - Default variation",
    Group32Var1 -> "Analog Input Event - 32-bit without time",
    Group32Var2 -> "Analog Input Event - 16-bit without time",
    Group32Var3 -> "Analog Input Event - 32-bit with time",
    Group32Var4 -> "Analog Input Event - 16-bit with time",
    Group32Var5 -> "Analog Input Event - Single-precision floating point without time",
    Group32Var6 -> "Analog Input Event - Double-precision floating point without time",
    Group32Var7 -> "Analog Input Event - Single-precision floating point with time",
    Group32Var8 -> "Analog Input Event - Double-precision floating point with time",
    Group33Var0 -> "Frozen Analog Input Event - Default variation",
    Group33Var1 -> "Frozen Analog Input Event - 32-bit without time",
    Group33Var2 -> "Frozen Analog Input Event - 16-bit without time",
    Group33Var3 -> "Frozen Analog Input Event - 32-bit with time",
    Group33Var4 -> "Frozen Analog Input Event - 16-bit with time",
    Group33Var5 -> "Frozen Analog Input Event - Single-precision floating point without time",
    Group33Var6 -> "Frozen Analog Input Event - Double-precision floating point without time",
    Group33Var7 -> "Frozen Analog Input Event - Single-precision floating point with time",
    Group33Var8 -> "Frozen Analog Input Event - Double-precision floating point with time",
    Group34Var0 -> "Analog Input Reporting Deadband - Default variation",
    Group34Var3 -> "Analog Input Reporting Deadband - Single-precision floating point",
    Group40Var0 -> "Analog Output Status - Default variation",
    Group40Var1 -> "Analog Output Status - 32-bit with flags",
    Group40Var2 -> "Analog Output Status - 16-bit with flags",
    Group40Var3 -> "Analog Output Status - Single-precision floating point with flags",
    Group40Var4 -> "Analog Output Status - Double-precision floating point with flags",
    Group41Var1 -> "Analog Output - 32-bit",
    Group41Var2 -> "Analog Output - 16-bit",
    Group41Var3 -> "Analog Output - Single-precision floating point",
    Group41Var4 -> "Analog Output - Double-precision floating point",
    Group42Var0 -> "Analog Output Event - Default variation",
    Group42Var1 -> "Analog Output Event - 32-bit without time",
    Group42Var2 -> "Analog Output Event - 16-bit without time",
    Group42Var3 -> "Analog Output Event - 32-bit with time",
    Group42Var4 -> "Analog Output Event - 16-bit with time",
    Group42Var5 -> "Analog Output Event - Single-precision floating point without time",
    Group42Var6 -> "Analog Output Event - Double-precision floating point without time",
    Group42Var7 -> "Analog Output Event - Single-precision floating point with time",
    Group42Var8 -> "Analog Output Event - Double-precision floating point with time",
    Group43Var1 -> "Analog Output Command Event - 32-bit without time",
    Group43Var2 -> "Analog Output Command Event - 16-bit without time",
    Group43Var3 -> "Analog Output Command Event - 32-bit with time",
    Group43Var4 -> "Analog Output Command Event - 16-bit with time",
    Group43Var5 -> "Analog Output Command Event - Single-precision floating point without time",
    Group43Var6 -> "Analog Output Command Event - Double-precision floating point without time",
    Group43Var7 -> "Analog Output Command Event - Single-precision floating point with time",
    Group43Var8 -> "Analog Output Command Event - Double-precision floating point with time",
    Group50Var1 -> "Time and Date - Absolute time",
    Group50Var3 -> "Time and Date - Absolute time at last recorded time",
    Group51Var1 -> "Time and date CTO - Absolute time, synchronized",
    Group51Var2 -> "Time and date CTO - Absolute time, unsynchronized",
    Group52Var1 -> "Time delay - Coarse",
    Group52Var2 -> "Time delay - Fine",
    Group60Var1 -> "Class objects - Class 0 data",
    Group60Var2 -> "Class objects - Class 1 data",
    Group60Var3 -> "Class objects - Class 2 data",
    Group60Var4 -> "Class objects - Class 3 data",
    Group70Var2 -> "File control - authentication",
    Group70Var3 -> "File control - file command",
    Group70Var4 -> "File control - file command status",
    Group70Var5 -> "File control - file transport",
    Group70Var6 -> "File control - file transport status",
    Group70Var7 -> "File control - file descriptor",
    Group70Var8 -> "File control - file specification string",
    Group80Var1 -> "Internal Indications - Packed format",
    Group102Var0 -> "Unsigned Integer - Default Variation",
    Group110AnyVar -> "Octet String",
    Group111AnyVar -> "Octet String Event",
  )

  def description(v: Variation): String = descriptions.getOrElse(v, v.fullDesc)
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
          s".push(${quoted(FfiVariation.schemaName(v))}, ${quoted(FfiVariation.description(v))})?".eol
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
