package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.groups.{Group110, Group111}
import dev.gridio.dnp3.codegen.render._

object AllObjectsVariationModule extends Module {

  override def lines(implicit indent: Indentation) : Iterator[String] = {
      "use crate::app::gen::variations::variation::Variation;".eol ++
      space ++
      variationEnumDefinition ++
      space ++
      variationEnumImpl
  }

  private def variationEnumDefinition(implicit indent: Indentation) : Iterator[String] = {

    "#[derive(Copy, Clone, Debug, PartialEq)]".eol ++
      bracket("pub(crate) enum AllObjectsVariation") {
        variations.iterator.map(v => s"${v.name},")
      }

  }

  private def variationEnumImpl(implicit indent: Indentation) : Iterator[String] = {

    def getMatcher(v: Variation) : String = v match {
      case _ : SizedByVariation =>   s"Variation::${v.parent.name}(0) => Some(AllObjectsVariation::${v.name}),"
      case _ => s"Variation::${v.name} => Some(AllObjectsVariation::${v.name}),"
    }

    bracket("impl AllObjectsVariation") {
      bracket("pub(crate) fn get(v: Variation) -> Option<AllObjectsVariation>") {
        bracket("match v") {
          variations.map(getMatcher) ++ "_ => None,".eol
        }
      }
    }

  }

  def variations : Iterator[Variation] = {
    ObjectGroup.allVariations.iterator.collect {
      case v : ClassData => v
      case v : AnyVariation => v
      case v : FixedSize if v.parent.groupType.isStatic || v.parent.groupType.isEvent => v
      case v if v.parent == Group110 || v.parent == Group111 => v
    }
  }

}
