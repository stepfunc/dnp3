package dev.gridio.dnp3.codegen

import java.nio.file.{FileSystems, Path}

import dev.gridio.dnp3.codegen.render._
import dev.gridio.dnp3.codegen.render.modules._

object Main {

  private val appPath: Path = FileSystems.getDefault.getPath("../src/app/")
  private val implPath: Path = appPath.resolve("gen");

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
    (ConversionsModule, implPath.resolve("conversion.rs"))
  )

  def main(args: Array[String]): Unit = {
    modules.foreach { case (m,p) => writeTo(p)(m) }
  }

}
