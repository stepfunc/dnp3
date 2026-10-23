// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.groups.{AllAttributesRequest, Group0, Group110, Group111, SpecificAttribute}
import dev.gridio.dnp3.codegen.render.*
import dev.gridio.dnp3.codegen.render.given_Conversion_String_RenderString

object AllObjectsVariationModule extends Module {

  override def lines(implicit indent: Indentation) : Iterator[String] = {
      "use crate::app::variations::Variation;".eol ++
      space ++
      variationDefinition ++
      space ++
      variationImpl
  }

  private def variationDefinition(implicit indent: Indentation) : Iterator[String] = {
    "/// A variation that an all-objects header outside a READ can name".eol ++
    "#[derive(Copy, Clone, Debug, PartialEq)]".eol ++
    "pub(crate) struct AllObjectsVariation(Variation);".eol
  }

  private def variationImpl(implicit indent: Indentation) : Iterator[String] = {

    def pattern(v: Variation) : String = v match {
      case _ : SizedByVariation => s"Variation::${v.parent.name}(0)"
      case SpecificAttribute => s"Variation::${v.parent.name}(_)"
      case _ => s"Variation::${v.name}"
    }

    val patterns = variations.map(pattern).toList

    bracket("impl AllObjectsVariation") {
      bracket("pub(crate) fn get(v: Variation) -> Option<AllObjectsVariation>") {
        bracket("match v") {
          patterns.headOption.iterator ++
          indent {
            patterns.drop(1).iterator.map(p => s"| ${p}")
          } ++
          indent {
            "=> Some(AllObjectsVariation(v)),".eol
          } ++
          "_ => None,".eol
        }
      } ++ space ++
      bracket("pub(crate) fn variation(self) -> Variation") {
        "self.0".eol
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
