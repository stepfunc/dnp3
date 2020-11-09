package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.groups._
import dev.gridio.dnp3.codegen.render._

object PrefixedVariationModule extends Module {

  override def lines(implicit indent: Indentation) : Iterator[String] = {
      "use crate::app::variations::*;".eol ++
      "use crate::app::parse::count::CountSequence;".eol ++
      "use crate::util::cursor::ReadCursor;".eol ++
      "use crate::app::parse::parser::*;".eol ++
      "use crate::app::parse::traits::{FixedSize, Index};".eol ++
      "use crate::app::parse::prefix::Prefix;".eol ++
      "use crate::app::parse::bytes::PrefixedBytesSequence;".eol ++
      "use crate::app::measurement::Time;".eol ++
      "use crate::master::handle::{ReadHandler, HeaderInfo};".eol ++
      "use crate::app::parse::error::ObjectParseError;".eol ++
      space ++
      enumDefinition ++
      space ++
      enumImpl
  }

  private def enumDefinition(implicit indent: Indentation) : Iterator[String] = {

    def definition(v : Variation) : Iterator[String] = {
      v match {
        case _ : SizedByVariation =>{
          "Group111VarX(u8, PrefixedBytesSequence<'a, I>),".eol
        }
        case _ => s"${v.name}(CountSequence<'a, Prefix<I, ${v.name}>>),".eol
      }
    }

    "#[derive(Debug, PartialEq)]".eol ++
      bracket("pub(crate) enum PrefixedVariation<'a, I> where I : FixedSize + Index + std::fmt::Display") {
        variations.iterator.flatMap(v =>  commented(v.fullDesc).eol ++ definition(v))
      }

  }

  private def enumImpl(implicit indent: Indentation) : Iterator[String] = {

    def parseMatcher(v: Variation) : Iterator[String] = v match {
      case Group111AnyVar => {
        "Variation::Group111(0) => Err(ObjectParseError::ZeroLengthOctetData),".eol ++
          "Variation::Group111(x) => Ok(PrefixedVariation::Group111VarX(x, PrefixedBytesSequence::parse(x, count, cursor)?)),".eol
      }
      case _ => {
        s"Variation::${v.name} => Ok(PrefixedVariation::${v.name}(CountSequence::parse(count, cursor)?)),".eol
      }
    }
    def fmtMatcher(v: Variation): Iterator[String] = v match {
      case _ : SizedByVariation => {
        s"PrefixedVariation::${v.parent.name}VarX(_,seq) =>  format_indexed_items(f, seq.iter()),".eol
      }
      case _ : FixedSize => {
        s"PrefixedVariation::${v.name}(seq) => format_prefixed_items(f, seq.iter()),".eol
      }
    }

    def infoMatcher(v: Variation) : Iterator[String] = {
      v match {
        case _ : SizedByVariation =>  {
          s"PrefixedVariation::${v.parent.name}VarX(x, _) =>  HeaderInfo::new(Variation::${v.parent.name}(*x), I::COUNT_AND_PREFIX_QUALIFIER),".eol
        }
        case _ =>  {
          s"PrefixedVariation::${v.name}(_) => HeaderInfo::new(Variation::${v.name}, I::COUNT_AND_PREFIX_QUALIFIER),".eol
        }
      }
    }


    def extractMatcher(v: Variation) : Iterator[String] = {
      def getName : String = v.parent.groupType match {
        case GroupType.BinaryEvent => "binary"
        case GroupType.DoubleBinaryEvent => "double_bit_binary"
        case GroupType.BinaryOutputEvent => "binary_output_status"
        case GroupType.CounterEvent => "counter"
        case GroupType.FrozenCounterEvent => "frozen_counter"
        case GroupType.AnalogEvent => "analog"
        case GroupType.AnalogOutputEvent => "analog_output_status"
        case GroupType.AnalogOutputCommandEvent => "analog_output_command"
        case GroupType.BinaryOutputCommandEvent => "binary_output_command"
        case _ => throw new Exception("unhandled variation")
      }

      v match {
        case _ if v.parent.groupType == GroupType.Command => {
          bracket(s"PrefixedVariation::${v.name}(_) =>") {
            "false // command".eol
          }
        }
        case Group111AnyVar => {
          bracket(s"PrefixedVariation::Group111VarX(_, seq) =>") {
            parenSemi("handler.handle_octet_string") {
              "self.get_header_info(),".eol ++
              "&mut seq.iter().map(|x| (x.0, x.1.widen_to_u16()))".eol
            } ++ "true".eol
          }
        }
        case Group2Var3 => {
          bracket(s"PrefixedVariation::${v.name}(seq) =>") {
            parenSemi("handler.handle_binary") {
              "self.get_header_info(),".eol ++
              "&mut seq.iter().map( |x| (x.value.to_measurement(cto), x.index.widen_to_u16()))".eol
            } ++ "true".eol
          }
        }
        case Group4Var3 => {
          bracket(s"PrefixedVariation::${v.name}(seq) =>") {
            parenSemi("handler.handle_double_bit_binary") {
              "self.get_header_info(),".eol ++
              "&mut seq.iter().map( |x| (x.value.to_measurement(cto), x.index.widen_to_u16()))".eol
            } ++ "true".eol
          }
        }
        case _ => {
          bracket(s"PrefixedVariation::${v.name}(seq) =>") {
            parenSemi(s"handler.handle_${getName}") {
              "self.get_header_info(),".eol ++
              "&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))".eol
            } ++ "true".eol
          }
        }
      }
    }

    bracket("impl<'a, I> PrefixedVariation<'a, I> where I : FixedSize + Index + std::fmt::Display") {
      bracket("pub(crate) fn parse(v: Variation, count: u16, cursor: &mut ReadCursor<'a>) -> Result<PrefixedVariation<'a, I>, ObjectParseError>") {
        bracket("match v") {
          variations.flatMap(parseMatcher) ++ "_ => Err(ObjectParseError::InvalidQualifierForVariation(v, I::COUNT_AND_PREFIX_QUALIFIER)),".eol
        }
      } ++ space ++
        bracket("pub(crate) fn format_objects(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result") {
          bracket("match self") {
            variations.flatMap(fmtMatcher).iterator
          }
        } ++ space ++
      bracket("pub(crate) fn extract_measurements_to(&self, cto: Option<Time>, handler: &mut dyn ReadHandler) -> bool") {
        bracket("match self") {
          variations.flatMap(extractMatcher).iterator
        }
      } ++ space ++
        bracket("pub(crate) fn get_header_info(&self) -> HeaderInfo") {
          bracket("match self") {
            variations.flatMap(infoMatcher).iterator
          }
        }
    }
  }

  def variations : Iterator[Variation] = {
    ObjectGroup.allVariations.iterator.collect {
      case v : SizedByVariation if v.parent == Group111 => v
      case v : FixedSize if v.parent.groupType.isEvent || v.parent.groupType == GroupType.Command => v
    }
  }

}
