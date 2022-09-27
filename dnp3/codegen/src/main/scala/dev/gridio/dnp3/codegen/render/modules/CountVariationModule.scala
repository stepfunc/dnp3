package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.groups.{Group111, Group60Var1}
import dev.gridio.dnp3.codegen.render._

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
        case _ : SizedByVariation => {
            s"${v.parent.name}Var0,".eol ++
            s"${v.parent.name}VarX(u8),".eol
        }
        case _ => s"${v.name},".eol
      }
    }

    "#[derive(Debug, PartialEq)]".eol ++
      bracket("pub(crate) enum CountVariation<'a>") {
        variations.iterator.flatMap(v =>  commented(v.fullDesc).eol ++ definition(v))
      }

  }

  private def enumImpl(implicit indent: Indentation) : Iterator[String] = {

    def parseMatcher(v : Variation) : Iterator[String] = {
      v match {
        case v : FixedSize if v.parent.groupType == GroupType.Time => s"Variation::${v.name} => Ok(CountVariation::${v.name}(CountSequence::parse(count, cursor)?)),".eol
        case _ : SizedByVariation => {
            s"Variation::${v.parent.name}(0) => Ok(CountVariation::${v.parent.name}Var0),".eol ++
            s"Variation::${v.parent.name}(x) => Ok(CountVariation::${v.parent.name}VarX(x)),".eol
        }
        case _ => s"Variation::${v.name} => Ok(CountVariation::${v.name}),".eol
      }
    }
    def fmtMatcher(v : Variation) : Iterator[String] = {
      v match {
        case v : FixedSize if v.parent.groupType == GroupType.Time => s"CountVariation::${v.name}(seq) => format_count_of_items(f, seq.iter()),".eol
        case _ : SizedByVariation => {
            s"CountVariation::${v.parent.name}Var0 => Ok(()),".eol ++
            s"CountVariation::${v.parent.name}VarX(_) => Ok(()),".eol
        }
        case _ => s"CountVariation::${v.name} => Ok(()),".eol
      }
    }

    bracket("impl<'a> CountVariation<'a>") {
      bracket("pub(crate) fn parse(v: Variation, qualifier: QualifierCode, count: u16, cursor: &mut ReadCursor<'a>) -> Result<CountVariation<'a>, ObjectParseError>") {
        bracket("match v") {
          variations.flatMap(parseMatcher) ++ "_ => Err(ObjectParseError::InvalidQualifierForVariation(v, qualifier)),".eol
        }
      } ++ space ++
      bracket("pub(crate) fn format_objects(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result") {
        bracket("match self") {
          variations.flatMap(fmtMatcher)
        }
      }
    }

  }

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
