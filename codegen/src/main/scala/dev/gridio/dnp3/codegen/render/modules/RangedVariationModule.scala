package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.render._

object RangedVariationModule {

  private def enumDefinition(implicit indent: Indentation) : Iterator[String] = {

    def getFixedVarDefinition(v: FixedSize) : String = {
      s"${v.name}(RangedSequence<'a, ${v.name}>),"
    }

    def getAnyVarDefinition(v: AnyVariation) : String = {
      s"${v.name},"
    }

    bracket("pub enum RangedVariation<'a>") {
      ObjectGroup.allVariations.iterator.collect {
        case v : AnyVariation if v.parent.isStaticGroup => getAnyVarDefinition(v)
        case v : FixedSize if v.parent.isStaticGroup => getFixedVarDefinition(v)
      }
    }
  }

  def file : Iterator[String] = {
    implicit val ident : Indentation = SpacedIndent

    Header() ++
    space ++
    "use crate::app::range::RangedSequence;".eol ++
    "use crate::app::variations::fixed::*;".eol ++
    space ++
    enumDefinition
  }
}
