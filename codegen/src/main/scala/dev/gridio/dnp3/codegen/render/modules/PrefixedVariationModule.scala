package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.render._

object PrefixedVariationModule extends Module {

  override def lines(implicit indent: Indentation) : Iterator[String] = {
      "use crate::app::gen::variations::gv::Variation;".eol ++
      "use crate::app::count::CountSequence;".eol ++
      "use crate::app::gen::variations::fixed::*;".eol ++
      "use crate::util::cursor::ReadCursor;".eol ++
      "use crate::app::parser::ParseError;".eol ++
      "use crate::app::header::FixedSize;".eol ++
      "use crate::app::prefix::Prefix;".eol ++
      space ++
      enumDefinition ++
      space ++
      enumImpl
  }

  private def enumDefinition(implicit indent: Indentation) : Iterator[String] = {

    "#[derive(Debug, PartialEq)]".eol ++
      bracket("pub enum PrefixedVariation<'a, I> where I : FixedSize") {
        variations.iterator.map(v => s"${v.name}(CountSequence<'a, Prefix<I, ${v.name}>>),")
      }

  }

  private def enumImpl(implicit indent: Indentation) : Iterator[String] = {

    bracket("impl<'a, I> PrefixedVariation<'a, I> where I : FixedSize") {
      "#[rustfmt::skip]".eol ++
      bracket("pub fn parse(v: Variation, count: u16, cursor: &mut ReadCursor<'a>) -> Result<PrefixedVariation<'a, I>, ParseError>") {
        bracket("match v") {
          variations.map { v =>
            s"Variation::${v.name} => Ok(PrefixedVariation::${v.name}(CountSequence::parse(count, cursor)?)),"
          } ++ "_ => Err(ParseError::InvalidQualifierForVariation(v)),".eol
        }
      }
    }

  }

  def variations : Iterator[Variation] = {
    ObjectGroup.allVariations.iterator.collect {
      case v : FixedSize if v.parent.groupType == GroupType.Event => v
    }
  }

}
