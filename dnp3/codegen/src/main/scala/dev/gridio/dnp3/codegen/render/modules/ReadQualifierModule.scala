// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.groups.SpecificAttribute
import dev.gridio.dnp3.codegen.render.*
import dev.gridio.dnp3.codegen.render.given_Conversion_String_RenderString

// a READ request carries no objects, so its headers are a variation and a qualifier: this module
// renders which variations a READ can carry with each qualifier
object ReadQualifierModule extends Module {

  override def lines(implicit indent: Indentation) : Iterator[String] = {
      "use crate::app::gen::all::AllObjectsVariation;".eol ++
      "use crate::app::parse::parser::ReadQualifier;".eol ++
      "use crate::app::variations::Variation;".eol ++
      space ++
      readQualifierImpl
  }

  private def readQualifierImpl(implicit indent: Indentation) : Iterator[String] = {

    def pattern(v: Variation, sizedByVariation: String) : String = v match {
      case _ : SizedByVariation => s"Variation::${v.parent.name}(${sizedByVariation})"
      case SpecificAttribute => s"Variation::${v.parent.name}(_)"
      case _ => s"Variation::${v.name}"
    }

    // a sized-by-variation group can be read by range only in variation 0, and by count in any
    // variation
    def accepts(qualifier: String, variations: Iterator[Variation], sizedByVariation: String) : Iterator[String] = {
      val patterns = variations.map(v => pattern(v, sizedByVariation)).toList
      s"${qualifier} => matches!(".eol ++ indent {
        "v,".eol ++
        patterns.iterator.zipWithIndex.map { case (p, i) => if (i == 0) p else s"| ${p}" }
      } ++ "),".eol
    }

    bracket("impl ReadQualifier") {
      "/// true if a READ request can carry the variation with this qualifier".eol ++
      bracket("pub(crate) fn accepts(self, v: Variation) -> bool") {
        bracket("match self") {
          "// a READ can name the same variations with all objects as any other function".eol ++
          "ReadQualifier::AllObjects => AllObjectsVariation::get(v).is_some(),".eol ++
          accepts("ReadQualifier::Range(_, _)", RangedVariationModule.variations.iterator, "0") ++
          accepts("ReadQualifier::Count(_, _)", CountVariationModule.variations, "_")
        }
      }
    }

  }

}
