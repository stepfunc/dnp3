package dev.gridio.dnp3.codegen

import java.nio.file.{FileSystems, Path}

import dev.gridio.dnp3.codegen.render.Module
import dev.gridio.dnp3.codegen.render.modules._

object Main {

  val appGenPath: Path = FileSystems.getDefault.getPath("../src/app/gen")
  val variationsPath: Path = appGenPath.resolve("variations")

  def modules : List[(Module, Path)] = List(
    (ProtocolEnums,  appGenPath.resolve("enums.rs")),
    (FixedSizeVariationModule,  variationsPath.resolve("fixed.rs")),
    (RangedVariationModule,  variationsPath.resolve("ranged.rs")),
    (AllObjectsVariationModule,  variationsPath.resolve("all.rs")),
    (VariationEnumModule, variationsPath.resolve("variation.rs")),
    (CountVariationModule, variationsPath.resolve("count.rs")),
    (PrefixedVariationModule, variationsPath.resolve("prefixed.rs")),
    (ConversionsModule, appGenPath.resolve("conversion.rs"))
  )



  def main(args: Array[String]): Unit = {
    modules.foreach { case (m,p) => writeTo(p)(m) }
  }

}
