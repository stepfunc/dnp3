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

    def definition(v : Variation): Iterator[String] = {
      v match {
        case v : FixedSize if v.parent.groupType == GroupType.Time => s"${v.name}(CountSequence<'a, ${v.name}>),".eol
        case _ : SizedByVariation => s"${v.parent.name}Var0,".eol
        case _ => s"${v.name},".eol
      }
    }

    "#[derive(Debug, PartialEq)]".eol ++
      bracket("pub(crate) enum CountVariation<'a>") {
        nonReadVariations.flatMap(v =>  commented(v.fullDesc).eol ++ definition(v))
      }

  }

  private def enumImpl(implicit indent: Indentation) : Iterator[String] = {

    def nonReadMatcher(v : Variation) : Iterator[String] = {
      v match {
        case v : FixedSize if isTime(v) => s"Variation::${v.name} => Ok(CountVariation::${v.name}(CountSequence::parse(count, cursor)?)),".eol
        case _ : SizedByVariation => s"Variation::${v.parent.name}(0) => Ok(CountVariation::${v.parent.name}Var0),".eol
        case _ => s"Variation::${v.name} => Ok(CountVariation::${v.name}),".eol
      }
    }

    def variationMatcher(v : Variation) : Iterator[String] = {
      v match {
        case v : FixedSize if isTime(v) => s"CountVariation::${v.name}(_) => Variation::${v.name},".eol
        case _ : SizedByVariation => s"CountVariation::${v.parent.name}Var0 => Variation::${v.parent.name}(0),".eol
        case _ => s"CountVariation::${v.name} => Variation::${v.name},".eol
      }
    }

    def fmtMatcher(v : Variation) : Iterator[String] = {
      v match {
        case v : FixedSize if isTime(v) => s"CountVariation::${v.name}(seq) => format_count_of_items(f, seq.iter()),".eol
        case _ : SizedByVariation => s"CountVariation::${v.parent.name}Var0 => Ok(()),".eol
        case _ => s"CountVariation::${v.name} => Ok(()),".eol
      }
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

  // outside a READ, a count of a variation with a known object size would have to be followed by
  // the objects, which only the time objects may carry
  private def nonReadVariations : Iterator[Variation] = {
    variations.filter {
      case v : FixedSize => isTime(v)
      case _ => true
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
