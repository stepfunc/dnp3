package dev.gridio.dnp3.codegen

import dev.gridio.dnp3.codegen.model.{FixedSize, ObjectGroup}
import dev.gridio.dnp3.codegen.render.{FixedSizeVariation, Indentation, SpacedIndent}

object Main {

  def main(args: Array[String]): Unit = {
    implicit val indent : Indentation = SpacedIndent

    ObjectGroup.all.foreach { g=>
      g.variations.foreach {
        case x: FixedSize => FixedSizeVariation.lines(x).foreach(println)
        case _ =>
      }
    }



  }

}
