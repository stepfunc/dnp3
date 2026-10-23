// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen

import java.nio.file.{FileSystems, Path}

import dev.gridio.dnp3.codegen.render.*
import dev.gridio.dnp3.codegen.render.given
import dev.gridio.dnp3.codegen.render.given_Conversion_String_RenderString
import dev.gridio.dnp3.codegen.render.modules.*
import dev.gridio.dnp3.codegen.writeTo

object Main {

  private val appPath: Path = FileSystems.getDefault.getPath("../src/app/")
  private val implPath: Path = appPath.resolve("gen");
  private val ffiPath: Path = FileSystems.getDefault.getPath("../../ffi/")

  private object CommonUseStatements extends Module {
    override def lines(implicit indentation: Indentation): Iterator[String] = {
      "use crate::app::parse::traits::{FixedSize, FixedSizeVariation};".eol ++
      "use crate::app::control::{CommandStatus, ControlCode};".eol ++
      "use crate::app::Timestamp;".eol ++
      "use crate::app::measurement::*;".eol ++
      space ++
      "use scursor::*;".eol
    }
  }

  def modules : List[(Module, Path)] = List(
    // these have some publicly exported stuff
    (EnumModule.application,  appPath.resolve("app_enums.rs")),
    (EnumModule.control,  appPath.resolve("control_enums.rs")),
    (CommonUseStatements ++ VariationEnumModule ++ FixedSizeVariationModule,  appPath.resolve("variations.rs")),
    // these don't contain any publicly exported stuff
    (RangedVariationModule,  implPath.resolve("ranged.rs")),
    (AllObjectsVariationModule,  implPath.resolve("all.rs")),
    (CountVariationModule, implPath.resolve("count.rs")),
    (PrefixedVariationModule, implPath.resolve("prefixed.rs")),
    (ReadQualifierModule, implPath.resolve("read.rs")),
    (ConversionsModule, implPath.resolve("conversion.rs")),
    // the FFI Variation enum and its conversions
    (FfiVariationSchemaModule, ffiPath.resolve("dnp3-schema/src/variation.rs")),
    (FfiVariationConversionModule, ffiPath.resolve("dnp3-ffi/src/variation.rs"))
  )

  def main(args: Array[String]): Unit = {
    modules.foreach { case (m,p) => writeTo(p)(m) }
  }

}
