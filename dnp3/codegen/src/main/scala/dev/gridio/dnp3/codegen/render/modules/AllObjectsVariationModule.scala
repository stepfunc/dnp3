package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.groups.{AllAttributesRequest, Group0, Group110, Group111, SpecificAttribute}
import dev.gridio.dnp3.codegen.render._

object AllObjectsVariationModule extends Module {

  override def lines(implicit indent: Indentation) : Iterator[String] = {
      "use crate::app::variations::Variation;".eol ++
      space ++
      variationEnumDefinition ++
      space ++
      variationEnumImpl
  }

  private def variationEnumDefinition(implicit indent: Indentation) : Iterator[String] = {

    def render(v: Variation) : String = {
      v match {
        case SpecificAttribute => s"${v.parent.name}(u8),"
        case _ =>   s"${v.name},"
      }
    }

    "#[derive(Copy, Clone, Debug, PartialEq)]".eol ++
      bracket("pub(crate) enum AllObjectsVariation") {
        variations.iterator.map(render)
      }

  }

  private def variationEnumImpl(implicit indent: Indentation) : Iterator[String] = {

    def getMatcher(v: Variation) : String = v match {
      case _ : SizedByVariation =>   s"Variation::${v.parent.name}(0) => Some(AllObjectsVariation::${v.name}),"
      case SpecificAttribute => s"Variation::${v.parent.name}(var) => Some(AllObjectsVariation::${v.parent.name}(var)),"
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
      case v : SingleBitField => v
      case v : DoubleBitField => v
      case v : ClassData => v
      case v : AnyVariation if v.parent.groupType != GroupType.Command => v
      case v : FixedSize if v.parent.groupType.isStatic || v.parent.groupType.isEvent => v
      case v : FixedSize if v.parent.groupType == GroupType.AnalogInputDeadband => v
      case v if v.parent == Group110 || v.parent == Group111 => v
      case AllAttributesRequest => AllAttributesRequest
      case SpecificAttribute => SpecificAttribute
    }
  }

}
