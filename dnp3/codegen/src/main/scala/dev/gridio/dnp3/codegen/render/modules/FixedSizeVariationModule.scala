package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.groups.{Group12Var1, Group1Var2, Group41Var1, Group41Var2, Group41Var3, Group41Var4}
import dev.gridio.dnp3.codegen.render._

object FixedSizeVariationModule extends Module {

  def lines(implicit indent : Indentation) : Iterator[String] = {

    def variations : List [FixedSize] = ObjectGroup.all.flatMap {
      g => g.variations.collect {
        case x : FixedSize => x
      }
    }

    spaced(variations.map(v => structDefinition(v)).iterator) ++
    space ++
    spaced(variations.map(v => implFixedSizedVariation(v)).iterator) ++
    space ++
    spaced(variations.map(v => implDisplay(v)).iterator) ++
    space ++
    spaced(variations.map(v => implHasVariation(v)).iterator)

  }

  private def getFieldType(f: FixedSizeFieldType) : String = {
    f match {
      case UInt8Field => "u8"
      case UInt16Field => "u16"
      case UInt32Field => "u32"
      case S16Field => "i16"
      case S32Field => "i32"
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
      case S16Field => "i16_le"
      case S32Field => "i32_le"
      case Float32Field => "f32_le"
      case Float64Field => "f64_le"
      case EnumFieldType(_) => "u8"
      case CustomFieldTypeU8(_) => "u8"
      case TimestampField => "u48_le"
      case _ => throw new Exception(s"Unhandled field type: ${f.toString}")
    }
  }

  private def visibility(gv : FixedSize): String = {
    val public = gv match {
      case Group12Var1 => true
      case Group41Var1 => true
      case Group41Var2 => true
      case Group41Var3 => true
      case Group41Var4 => true
      case _ => false
    }
    if(public) "" else "(crate)"
  }

  private def structDefinition(gv : FixedSize)(implicit indent: Indentation): Iterator[String] = {
    def field(f: FixedSizeField) : Iterator[String] = {
      Iterator(s"/// ${f.name} field of the variation") ++
      Iterator(s"pub${visibility(gv)} ${f.name}: ${getFieldType(f.typ)},")
    }

    val derives = if(gv.hasFloatingPoint) {
      "#[derive(Copy, Clone, Debug, PartialEq)]"
    } else {
      "#[derive(Copy, Clone, Debug, PartialEq, Eq)]"
    }

    commented(gv.fullDesc).eol ++ derives.eol ++
    bracket(s"pub${visibility(gv)} struct ${gv.name}") {
      gv.fields.flatMap(f => field(f)).iterator
    }
  }

  private def implDisplay(gv : FixedSize)(implicit indent: Indentation): Iterator[String] = {
    def fieldDisplayType(typ: FixedSizeFieldType): String = {
      typ match {
        case _ : EnumFieldType => "{:?}"
        case _ : CustomFieldTypeU8 => "{}"
        case _ => "{}"
      }
    }

    def fieldNames : String = {
      quoted(gv.fields.map( f=> s"${f.name}: ${fieldDisplayType(f.typ)}").mkString(" "))
    }

    def fieldArgExpression(f: FixedSizeField) : String = {

      def getFlagsType : String = {
        def binary = "BinaryFlagFormatter"
        def analog = "AnalogFlagFormatter"
        def counter = "CounterFlagFormatter"
        def binaryOutputStatus = "BinaryOutputStatusFlagFormatter"
        def doubleBitBinary = "DoubleBitBinaryFlagFormatter"

        gv.parent.groupType match {
          case GroupType.StaticBinary => binary
          case GroupType.BinaryEvent => binary
          case GroupType.AnalogOutputEvent => analog
          case GroupType.StaticAnalogOutputStatus => analog
          case GroupType.AnalogEvent => analog
          case GroupType.StaticAnalog => analog
          case GroupType.FrozenAnalogEvent => analog
          case GroupType.StaticFrozenAnalog => analog
          case GroupType.StaticCounter => counter
          case GroupType.CounterEvent => counter
          case GroupType.StaticFrozenCounter => counter
          case GroupType.FrozenCounterEvent => counter
          case GroupType.BinaryOutputEvent => binaryOutputStatus
          case GroupType.StaticBinaryOutputStatus => binaryOutputStatus
          case GroupType.StaticDoubleBinary => doubleBitBinary
          case GroupType.DoubleBinaryEvent => doubleBitBinary
          case GroupType.AnalogOutputCommandEvent => analog
          case GroupType.BinaryOutputCommandEvent => binary
          case _ => throw new Exception("unhandled group type")
        }
      }


      if(f.isFlags) {
        s"${getFlagsType}::new(self.flags)"
      } else {
        s"self.${f.name}"
      }
    }

    def fieldArgs : String = {
      gv.fields.map(fieldArgExpression).mkString(", ")
    }

    bracket(s"impl std::fmt::Display for ${gv.name}") {
      bracket("fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result") {
        s"write!(f, ${fieldNames}, ${fieldArgs})".eol
      }
    }
  }

  private def implHasVariation(gv : FixedSize)(implicit indent: Indentation): Iterator[String] = {
    bracket(s"impl FixedSizeVariation for ${gv.name}") {
      s"const VARIATION : Variation = Variation::${gv.name};".eol
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
        case _ : EnumFieldType => s"self.${f.name}.write(cursor)?;"
        case CustomFieldTypeU8(name) => write(".as_u8()")
        case TimestampField => s"self.${f.name}.write(cursor)?;"
        case _ => write("")
      }
    }

    def implRead : Iterator[String] = {
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
