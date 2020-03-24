package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.groups.{Group111, Group111AnyVar}
import dev.gridio.dnp3.codegen.render._

object PrefixedVariationModule extends Module {

  override def lines(implicit indent: Indentation) : Iterator[String] = {
      "use crate::app::gen::variations::gv::Variation;".eol ++
      "use crate::app::parse::count::CountSequence;".eol ++
      "use crate::app::gen::variations::fixed::*;".eol ++
      "use crate::util::cursor::ReadCursor;".eol ++
      "use crate::app::parse::parser::ParseError;".eol ++
      "use crate::app::parse::header::FixedSize;".eol ++
      "use crate::app::parse::prefix::Prefix;".eol ++
      "use crate::app::parse::bytes::PrefixedBytesSequence;".eol ++
      space ++
      enumDefinition ++
      space ++
      enumImpl
  }

  private def enumDefinition(implicit indent: Indentation) : Iterator[String] = {

    def definition(v : Variation) : Iterator[String] = v match {
      case _ : SizedByVariation =>{
        "Group111VarX(u8, PrefixedBytesSequence<'a, I>),".eol
      }
      case _ => s"${v.name}(CountSequence<'a, Prefix<I, ${v.name}>>),".eol
    }

    "#[derive(Debug, PartialEq)]".eol ++
      bracket("pub enum PrefixedVariation<'a, I> where I : FixedSize") {
        variations.iterator.flatMap(definition)
      }

  }

  private def enumImpl(implicit indent: Indentation) : Iterator[String] = {

    def matcher(v: Variation) : Iterator[String] = v match {
      case Group111AnyVar => {
        "Variation::Group111(0) => Err(ParseError::ZeroLengthOctetData),".eol ++
        "Variation::Group111(x) => Ok(PrefixedVariation::Group111VarX(x, PrefixedBytesSequence::parse(x, count, cursor)?)),".eol
      }
      case _ => {
        s"Variation::${v.name} => Ok(PrefixedVariation::${v.name}(CountSequence::parse(count, cursor)?)),".eol
      }
    }

    bracket("impl<'a, I> PrefixedVariation<'a, I> where I : FixedSize") {
      "#[rustfmt::skip]".eol ++
      bracket("pub fn parse(v: Variation, count: u16, cursor: &mut ReadCursor<'a>) -> Result<PrefixedVariation<'a, I>, ParseError>") {
        bracket("match v") {
          variations.flatMap(matcher) ++ "_ => Err(ParseError::InvalidQualifierForVariation(v)),".eol
        }
      }
    }

  }

  def variations : Iterator[Variation] = {
    ObjectGroup.allVariations.iterator.collect {
      case v : SizedByVariation if v.parent == Group111 => v
      case v : FixedSize if v.parent.groupType == GroupType.Event || v.parent.groupType == GroupType.Command => v
    }
  }

}
