package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.render._

object RangedVariationModule extends Module {

  override def lines(implicit indent: Indentation) : Iterator[String] = {
    "use crate::app::range::{RangedSequence, Range};".eol ++
    "use crate::app::variations::fixed::*;".eol ++
      "use crate::app::variations::gv::Variation;".eol ++
      "use crate::util::cursor::ReadCursor;".eol ++
      "use crate::app::parser::ParseError;".eol ++
      space ++
      rangedVariationEnumDefinition ++
      space ++
      rangedVariationEnumImpl
  }

  private def rangedVariationEnumDefinition(implicit indent: Indentation) : Iterator[String] = {

    def getVarDefinition(v: Variation) : String = v match {
      case v : AnyVariation => s"${v.name},"
      case v : FixedSize => s"${v.name}(RangedSequence<'a, ${v.name}>),"
    }

    "#[derive(Debug, PartialEq)]".eol ++
      bracket("pub enum RangedVariation<'a>") {
        variations.iterator.map(getVarDefinition)
      }

  }

  private def rangedVariationEnumImpl(implicit indent: Indentation) : Iterator[String] = {

    def getNonReadVarDefinition(v: Variation) : String = v match {
      case v : AnyVariation => ""
      case v : FixedSize => "(RangedSequence::parse(range, cursor)?)"
    }

    def getReadVarDefinition(v: Variation) : String = v match {
      case v : AnyVariation => ""
      case v : FixedSize => "(RangedSequence::empty())"
    }

    bracket("impl<'a> RangedVariation<'a>") {
      "#[rustfmt::skip]".eol ++
      bracket("pub fn parse_non_read(v: Variation, range: Range, cursor: &mut ReadCursor<'a>) -> Result<RangedVariation<'a>, ParseError>") {
        bracket("match v") {
          variations.map { v =>
            s"Variation::${v.name} => Ok(RangedVariation::${v.name}${getNonReadVarDefinition(v)}),"
          } ++ "_ => Err(ParseError::InvalidQualifierAndObject),".eol
        }
      } ++ space ++
        bracket("pub fn parse_read(v: Variation) -> Result<RangedVariation<'a>, ParseError>") {
          bracket("match v") {
            variations.map { v =>
              s"Variation::${v.name} => Ok(RangedVariation::${v.name}${getReadVarDefinition(v)}),"
            } ++ "_ => Err(ParseError::InvalidQualifierAndObject),".eol
          }
        }
    }

  }

  def variations : Iterator[Variation] = {
    ObjectGroup.allVariations.iterator.collect {
      case v : AnyVariation if v.parent.isStaticGroup => v
      case v : FixedSize if v.parent.isStaticGroup => v
    }
  }

}
