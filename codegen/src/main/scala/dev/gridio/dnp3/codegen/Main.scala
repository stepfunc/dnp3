package dev.gridio.dnp3.codegen

import java.nio.file.{FileSystems, Path}

import dev.gridio.dnp3.codegen.render.FixedSizeVariation

object Main {

  val appGenPath: Path = FileSystems.getDefault.getPath("../src/app/variations")
  val fixedSizePath : Path = appGenPath.resolve("fixed.rs")

  def main(args: Array[String]): Unit = {


    writeTo(fixedSizePath)(FixedSizeVariation.file)



  }

}
