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

    "use crate::app::parse::traits::FixedSize;".eol ++
    "use crate::util::cursor::*;".eol ++
    "use crate::app::gen::enums::CommandStatus;".eol ++
    "use crate::app::types::{ControlCode, Timestamp};".eol ++
    space ++
    spaced(variations.map(v => structDefinition(v)).iterator) ++
    space ++
    spaced(variations.map(v => implFixedSizedVariation(v)).iterator)
  }

  private def getFieldType(f: FixedSizeFieldType) : String = {
    f match {
      case UInt8Field => "u8"
      case UInt16Field => "u16"
      case UInt32Field => "u32"
      case SInt16Field => "i16"
      case SInt32Field => "i32"
      case Float32Field => "f32"
      case Float64Field => "f64"
      case x : EnumFieldType => x.model.name
      case x : CustomFieldTypeU8 => x.structName
      case TimestampField => "Timestamp"
      case _ => throw new Exception(s"Unhandled field type: ${f.toString}")
    }
  }

  private def getCursorSuffix(f: FixedSizeFieldType) : String = {
    f match {
      case UInt8Field => "u8"
      case UInt16Field => "u16_le"
      case UInt32Field => "u32_le"
      case SInt16Field => "i16_le"
      case SInt32Field => "i32_le"
      case Float32Field => "f32_le"
      case Float64Field => "f64_le"
      case EnumFieldType(_) => "u8"
      case CustomFieldTypeU8(_) => "u8"
      case TimestampField => "u48_le"
      case _ => throw new Exception(s"Unhandled field type: ${f.toString}")
    }
  }

  private def structDefinition(gv : FixedSize)(implicit indent: Indentation): Iterator[String] = {
    commented(gv.fullDesc).eol ++
    "#[derive(Debug, PartialEq)]".eol ++
    bracket(s"pub struct ${gv.name}") {
      gv.fields.map(f => s"pub ${f.name}: ${getFieldType(f.typ)},").iterator
    }
  }

  private def implFixedSizedVariation(gv : FixedSize)(implicit indent: Indentation): Iterator[String] = {
    def readField(f : FixedSizeField) : String = {
      val inner = s"cursor.read_${getCursorSuffix(f.typ)}()?"
      f.typ match {
        case x : EnumFieldType => s"${x.model.name}::from(${inner})"
        case CustomFieldTypeU8(name) => s"${name}::from(${inner})"
        case TimestampField => s"Timestamp::new(${inner})"
        case _ => inner
      }
    }

    def writeField(f : FixedSizeField) : String = {
      def write(suffix: String) = s"cursor.write_${getCursorSuffix(f.typ)}(self.${f.name}${suffix})?;"
      f.typ match {
        case _ : EnumFieldType => write(".as_u8()")
        case CustomFieldTypeU8(name) => write(".as_u8()")
        case TimestampField => write(".value")
        case _ => write("")
      }
    }

    def implRead : Iterator[String] = {
      "#[rustfmt::skip]".eol ++
        bracket(s"fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError>") {
          paren("Ok") {
            bracket(s"${gv.name}") {
              gv.fields.iterator.flatMap { f =>
                s"${f.name}: ${readField(f)},".eol
              }
            }
          }
        }
    }

    def implWrite : Iterator[String] = {
      "#[rustfmt::skip]".eol ++
        bracket(s"fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError>") {
            gv.fields.iterator.flatMap { f =>
              writeField(f).eol
            } ++ "Ok(())".eol
        }
    }

    bracket(s"impl FixedSize for ${gv.name}") {
      s"const SIZE: u8 = ${gv.size};".eol ++
      implRead ++
      implWrite
    }
  }

}
