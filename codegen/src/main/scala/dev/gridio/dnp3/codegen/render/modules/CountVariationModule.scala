package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.render._

object CountVariationModule extends Module {

  override def lines(implicit indent: Indentation) : Iterator[String] = {
      "use crate::app::gen::variations::gv::Variation;".eol ++
      space ++
      enumDefinition ++
      space ++
      enumImpl
  }

  private def enumDefinition(implicit indent: Indentation) : Iterator[String] = {

    "#[derive(Debug, PartialEq)]".eol ++
      bracket("pub enum CountVariation") {
        variations.iterator.map(v => s"${v.name},")
      }

  }

  private def enumImpl(implicit indent: Indentation) : Iterator[String] = {

    bracket("impl CountVariation") {
      "#[rustfmt::skip]".eol ++
      bracket("pub fn get(v: Variation) -> Option<CountVariation>") {
        bracket("match v") {
          variations.map { v =>
            s"Variation::${v.name} => Some(CountVariation::${v.name}),"
          } ++ "_ => None,".eol
        }
      }
    }

  }

  def variations : Iterator[Variation] = {
    ObjectGroup.allVariations.iterator.collect {
      case v : FixedSize if v.parent.groupType == GroupType.Time => v
    }
  }

}
