package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.render._

object CountVariationModule extends Module {

  override def lines(implicit indent: Indentation) : Iterator[String] = {
      "use crate::app::variations::*;".eol ++ "use crate::app::QualifierCode;".eol ++
      "use crate::app::parse::count::CountSequence;".eol ++
      "use crate::app::parse::parser::*;".eol ++
      "use crate::util::cursor::ReadCursor;".eol ++
      "use crate::app::ObjectParseError;".eol ++
      space ++
      enumDefinition ++
      space ++
      enumImpl
  }

  private def enumDefinition(implicit indent: Indentation) : Iterator[String] = {

    def definition(v : Variation): Iterator[String] = {
      v match {
        case _ : FixedSize => s"${v.name}(CountSequence<'a, ${v.name}>),".eol
      }
    }

    "#[derive(Debug, PartialEq)]".eol ++
      bracket("pub(crate) enum CountVariation<'a>") {
        variations.iterator.flatMap(v =>  commented(v.fullDesc).eol ++ definition(v))
      }

  }

  private def enumImpl(implicit indent: Indentation) : Iterator[String] = {

    def parseMatcher(v : Variation) : String = {
      v match {
        case _ : FixedSize => s"Variation::${v.name} => Ok(CountVariation::${v.name}(CountSequence::parse(count, cursor)?)),"
      }
    }
    def fmtMatcher(v : Variation) : String = {
      v match {
        case _ : FixedSize => s"CountVariation::${v.name}(seq) => format_count_of_items(f, seq.iter()),"
      }
    }

    bracket("impl<'a> CountVariation<'a>") {
      bracket("pub(crate) fn parse(v: Variation, qualifier: QualifierCode, count: u16, cursor: &mut ReadCursor<'a>) -> Result<CountVariation<'a>, ObjectParseError>") {
        bracket("match v") {
          variations.map(parseMatcher) ++ "_ => Err(ObjectParseError::InvalidQualifierForVariation(v, qualifier)),".eol
        }
      } ++ space ++
      bracket("pub(crate) fn format_objects(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result") {
        bracket("match self") {
          variations.map(fmtMatcher).iterator
        }
      }
    }

  }

  def variations : Iterator[Variation] = {
    ObjectGroup.allVariations.iterator.flatMap { v =>
      v match {
        case v : FixedSize if v.parent.groupType == GroupType.Time => Some(v)
        case _ => None
      }
    }
  }

}
