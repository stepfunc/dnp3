package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.render._

object FixedSizeVariationModule extends Module {

  def lines(implicit indent : Indentation) : Iterator[String] = {

    def variations : List [FixedSize] = ObjectGroup.all.flatMap {
      g => g.variations.collect {
        case x : FixedSize => x
      }
    }

    "use crate::app::header::FixedSizeVariation;".eol ++
    "use crate::util::cursor::{ReadCursor, ReadError};".eol ++
    "use crate::app::gen::enums::CommandStatus;".eol ++
    space ++
    spaced(variations.iterator.map(v => structDefinition(v))) ++
    spaced(variations.iterator.map(v => implFixedSizedVariation(v)))
  }

  private def getFieldType(f: FixedSizeFieldType) : String = {
    f match {
      case UInt8Field => "u8"
      case UInt16Field => "u16"
      case UInt32Field => "u32"
      case UInt48Field => "u64"
      case SInt16Field => "i16"
      case SInt32Field => "i32"
      case Float32Field => "f32"
      case Float64Field => "f64"
      case x : EnumFieldType => x.model.name
      case _ => throw new Exception(s"Unhandled field type: ${f.toString}")
    }
  }

  private def getReadSuffix(f: FixedSizeFieldType) : String = {
    f match {
      case UInt8Field => "u8"
      case UInt16Field => "u16_le"
      case UInt32Field => "u32_le"
      case UInt48Field => "u48_le"
      case SInt16Field => "i16_le"
      case SInt32Field => "i32_le"
      case Float32Field => "f32_le"
      case Float64Field => "f64_le"
      case EnumFieldType(_) => "u8"
      case _ => throw new Exception(s"Unhandled field type: ${f.toString}")
    }
  }

  private def structDefinition(gv : FixedSize)(implicit indent: Indentation): Iterator[String] = {
    "#[derive(Debug, PartialEq)]".eol ++
    bracket(s"pub struct ${gv.name}") {
      gv.fields.map(f => s"pub ${f.name}: ${getFieldType(f.typ)},").iterator
    }
  }

  private def implFixedSizedVariation(gv : FixedSize)(implicit indent: Indentation): Iterator[String] = {
    def parseField(f : FixedSizeField) : String = {
      val inner = s"cursor.read_${getReadSuffix(f.typ)}()?"
      f.typ match {
        case x : EnumFieldType => s"${x.model.name}::from(${inner})"
        case _ => inner
      }
    }

    def implParse : Iterator[String] = {
      "#[rustfmt::skip]".eol ++
      bracket(s"fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError>") {
        paren("Ok") {
          bracket(s"${gv.name}") {
            gv.fields.iterator.flatMap { f =>
              s"${f.name}: ${parseField(f)},".eol
            }
          }
        }
      }
    }

    bracket(s"impl FixedSizeVariation for ${gv.name}") {
      s"const SIZE: u8 = ${gv.size};".eol ++
      implParse
    }
  }

  private def lines(gv : FixedSize)(implicit indent: Indentation): Iterator[String] = {
    structDefinition(gv) ++ space ++ implFixedSizedVariation(gv)
  }

  private def lines(gv: Seq[FixedSize])(implicit indent: Indentation): Iterator[String] = {
    spaced(gv.map(x => lines(x)).iterator)
  }

}
