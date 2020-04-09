package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.render._

object CountVariationModule extends Module {

  override def lines(implicit indent: Indentation) : Iterator[String] = {
      "use crate::app::gen::variations::variation::Variation;".eol ++
      "use crate::app::gen::variations::fixed::*;".eol ++
      "use crate::app::parse::count::CountSequence;".eol ++
      "use crate::app::parse::parser::*;".eol ++
      "use crate::util::cursor::ReadCursor;".eol ++
      "use crate::app::parse::error::ObjectParseError;".eol ++
      "use crate::app::gen::enums::QualifierCode;".eol ++
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
      bracket("pub enum CountVariation<'a>") {
        variations.iterator.flatMap(v =>  commented(v.fullDesc).eol ++ definition(v))
      }

  }

  private def enumImpl(implicit indent: Indentation) : Iterator[String] = {

    def parseMatcher(v : Variation) : String = {
      v match {
        case _ : FixedSize => s"Variation::${v.name} => Ok(CountVariation::${v.name}(CountSequence::parse(count, cursor)?)),"
      }
    }

    def logMatcher(v : Variation) : String = {
      v match {
        case _ : FixedSize => s"CountVariation::${v.name}(seq) => log_count_of_items(level, seq.iter()),"
      }
    }
    def fmtMatcher(v : Variation) : String = {
      v match {
        case _ : FixedSize => s"CountVariation::${v.name}(seq) => format_count_of_items(f, seq.iter()),"
      }
    }

    "#[rustfmt::skip]".eol ++
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
      /*
      ++ space ++
      bracket("pub(crate) fn log_objects(&self, level : log::Level)") {
        bracket("match self") {
          variations.map(logMatcher).iterator
        }
      }
       */
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
