package dev.gridio.dnp3.codegen

import java.nio.file.{FileSystems, Path}


import dev.gridio.dnp3.codegen.render.modules._

object Main {

  val appGenPath: Path = FileSystems.getDefault.getPath("../src/app/gen")
  val variationsPath: Path = appGenPath.resolve("variations")

  val enumsPath : Path = appGenPath.resolve("enums.rs")
  val fixedSizePath : Path = variationsPath.resolve("fixed.rs")
  val rangedPath : Path = variationsPath.resolve("ranged.rs")
  val gvPath : Path = variationsPath.resolve("gv.rs")



  def main(args: Array[String]): Unit = {
    writeTo(enumsPath)(ProtocolEnums)
    writeTo(fixedSizePath)(FixedSizeVariationModule)
    writeTo(gvPath)(VariationEnumModule)
    writeTo(rangedPath)(RangedVariationModule)
  }

}
