package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.render._

object AllObjectsVariationModule extends Module {

  override def lines(implicit indent: Indentation) : Iterator[String] = {
      "use crate::app::gen::variations::gv::Variation;".eol ++
      space ++
      variationEnumDefinition ++
      space ++
      variationEnumImpl
  }

  private def variationEnumDefinition(implicit indent: Indentation) : Iterator[String] = {

    "#[derive(Debug, PartialEq)]".eol ++
      bracket("pub enum AllObjectsVariation") {
        variations.iterator.map(v => s"${v.name},")
      }

  }

  private def variationEnumImpl(implicit indent: Indentation) : Iterator[String] = {

    bracket("impl AllObjectsVariation") {
      "#[rustfmt::skip]".eol ++
      bracket("pub fn get(v: Variation) -> Option<AllObjectsVariation>") {
        bracket("match v") {
          variations.map { v =>
            s"Variation::${v.name} => Some(AllObjectsVariation::${v.name}),"
          } ++ "_ => None,".eol
        }
      }
    }

  }

  def variations : Iterator[Variation] = {
    ObjectGroup.allVariations.iterator.collect {
      case v : ClassData => v
      case v : AnyVariation => v
      case v : FixedSize if v.parent.isStaticGroup || v.parent.isEventGroup => v
    }
  }

}
