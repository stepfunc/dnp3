package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.render._

object CountVariationModule extends Module {

  override def lines(implicit indent: Indentation) : Iterator[String] = {
      "use crate::app::gen::variations::gv::Variation;".eol ++
      "use crate::app::gen::variations::fixed::*;".eol ++
      "use crate::app::parse::count::CountSequence;".eol ++
      "use crate::app::parse::parser::ObjectParseError;".eol ++
      "use crate::util::cursor::ReadCursor;".eol ++
      "use crate::util::logging::*;".eol ++
      space ++
      enumDefinition ++
      space ++
      enumImpl
  }

  private def enumDefinition(implicit indent: Indentation) : Iterator[String] = {

    def definition(v : Variation): String = {
      v match {
        case _ : FixedSize => s"${v.name}(CountSequence<'a, ${v.name}>),"
      }
    }

    "#[derive(Debug, PartialEq)]".eol ++
      bracket("pub enum CountVariation<'a>") {
        variations.iterator.map(definition)
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

    bracket("impl<'a> CountVariation<'a>") {
      "#[rustfmt::skip]".eol ++
      bracket("pub fn parse(v: Variation, count: u16, cursor: &mut ReadCursor<'a>) -> Result<CountVariation<'a>, ObjectParseError>") {
        bracket("match v") {
          variations.map(parseMatcher) ++ "_ => Err(ObjectParseError::InvalidQualifierForVariation(v)),".eol
        }
      } ++ space ++
      bracket("pub fn log(&self, level : log::Level)") {
        bracket("match self") {
          variations.map(logMatcher).iterator
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
