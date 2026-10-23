// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.groups.{Group111, Group60Var1}
import dev.gridio.dnp3.codegen.render.*
import dev.gridio.dnp3.codegen.render.given_Conversion_String_RenderString

object CountVariationModule extends Module {

  override def lines(implicit indent: Indentation) : Iterator[String] = {
      "use crate::app::variations::*;".eol ++ "use crate::app::QualifierCode;".eol ++
      "use crate::app::parse::count::CountSequence;".eol ++
      "use crate::app::parse::parser::*;".eol ++
      "use crate::app::ObjectParseError;".eol ++
      space ++
      "use scursor::ReadCursor;".eol ++
      space ++
      enumDefinition ++
      space ++
      enumImpl
  }

  private def enumDefinition(implicit indent: Indentation) : Iterator[String] = {

    "#[derive(Debug, PartialEq)]".eol ++
      bracket("pub(crate) enum CountVariation<'a>") {
        nonReadVariations.flatMap(v =>  commented(v.fullDesc).eol ++ s"${v.name}(CountSequence<'a, ${v.name}>),".eol)
      }

  }

  private def enumImpl(implicit indent: Indentation) : Iterator[String] = {

    def nonReadMatcher(v : Variation) : Iterator[String] = {
      s"Variation::${v.name} => Ok(CountVariation::${v.name}(CountSequence::parse(count, cursor)?)),".eol
    }

    def variationMatcher(v : Variation) : Iterator[String] = {
      s"CountVariation::${v.name}(_) => Variation::${v.name},".eol
    }

    def fmtMatcher(v : Variation) : Iterator[String] = {
      s"CountVariation::${v.name}(seq) => format_count_of_items(f, seq.iter()),".eol
    }

    bracket("impl<'a> CountVariation<'a>") {
      bracket("pub(crate) fn parse_non_read(v: Variation, qualifier: QualifierCode, count: u16, cursor: &mut ReadCursor<'a>) -> Result<CountVariation<'a>, ObjectParseError>") {
        bracket("match v") {
          nonReadVariations.flatMap(nonReadMatcher) ++ "_ => Err(ObjectParseError::InvalidQualifierForVariation(v, qualifier)),".eol
        }
      } ++ space ++
      bracket("pub(crate) fn variation(&self) -> Variation") {
        bracket("match self") {
          nonReadVariations.flatMap(variationMatcher)
        }
      } ++ space ++
      bracket("pub(crate) fn format_objects(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result") {
        bracket("match self") {
          nonReadVariations.flatMap(fmtMatcher)
        }
      }
    }

  }

  private def isTime(v : Variation) : Boolean = v.parent.groupType == GroupType.Time

  // outside a READ, a count header must be followed by its objects, which only the time objects may
  // carry: the "any variation", class and sized-by-variation objects that a READ counts have none
  private def nonReadVariations : Iterator[Variation] = {
    variations.collect {
      case v : FixedSize if isTime(v) => v
    }
  }

  // the variations a count header can name, including those that only a READ can
  def variations : Iterator[Variation] = {
    ObjectGroup.allVariations.iterator.flatMap { v =>
      v match {
        case v : AnyVariation if v.parent.groupType.isEvent => Some(v)
        case v : FixedSize if v.parent.groupType.isEvent => Some(v)
        case v : SizedByVariation if v.parent.groupType.isEvent => Some(v)
        case v : ClassData if v != Group60Var1 => Some(v)
        case v : FixedSize if v.parent.groupType == GroupType.Time => Some(v)
        case _ => None
      }
    }
  }

}
