package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.render._

object RangedVariationModule extends Module {

  override def lines(implicit indent: Indentation) : Iterator[String] = {
    "use crate::app::range::RangedSequence;".eol ++
      "use crate::app::variations::fixed::*;".eol ++
      space ++
      enumDefinition
  }

  private def enumDefinition(implicit indent: Indentation) : Iterator[String] = {

    def getFixedVarDefinition(v: FixedSize) : String = {
      s"${v.name}(RangedSequence<'a, ${v.name}>),"
    }

    def getAnyVarDefinition(v: AnyVariation) : String = {
      s"${v.name},"
    }

    "#[derive(Debug, PartialEq)]".eol ++
    bracket("pub enum RangedVarData<'a>") {
      ObjectGroup.allVariations.iterator.collect {
        case v : AnyVariation if v.parent.isStaticGroup => getAnyVarDefinition(v)
        case v : FixedSize if v.parent.isStaticGroup => getFixedVarDefinition(v)
      }
    }
  }

}
