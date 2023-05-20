package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.enums.protocol._
import dev.gridio.dnp3.codegen.render._

object EnumModule {
  def application : Module = {
    new EnumModule(List(FunctionCode, QualifierCode))
  }
  def control : Module = {
    new EnumModule(List(CommandStatus, OpType, TripCloseCode))
  }
}

class EnumModule(enums: List[EnumModel]) extends Module {

  private def excludeWriteImpl(model : EnumModel) : Boolean = {
    model match  {
      case OpType => true
      case TripCloseCode => true
      case _ => false
    }
  }

  private def lines(model : EnumModel)(implicit indentation: Indentation) : Iterator[String] = {
    def values: Iterator[String] = {
      model.values.flatMap(v => s"///  ${v.comment} (value == ${model.render(v.value)})".eol ++ s"${v.name},".eol).iterator
    }

    def fromValueToOption: Iterator[String] = {
      "/// try to create the enum from the underlying value, returning None".eol ++
      "/// if the specified value is undefined".eol ++
      bracket("pub fn from(x: u8) -> Option<Self>") {
        bracket("match x") {
          model.values.iterator.map(v => s"${model.render(v.value)} => Some(${model.name}::${v.name}),") ++ "_ => None,".eol
        }
      }
    }

    def fromValue: Iterator[String] = {
      "/// create the enum from the underlying value".eol ++
      bracket("pub fn from(x: u8) -> Self") {
        bracket("match x") {
          model.values.iterator.map(v => s"${model.render(v.value)} => ${model.name}::${v.name},") ++ s"_ => ${model.name}::Unknown(x),".eol
        }
      }
    }

    def asValue: Iterator[String] = {
      def last : Iterator[String] = {
        if(model.captureUnknownValues) {
          s"${model.name}::Unknown(x) => x,".eol
        } else {
          Iterator.empty
        }
      }

      "/// convert the enum to its underlying value".eol ++
      bracket("pub fn as_u8(self) -> u8") {
        bracket("match self") {
          model.values.iterator.map(v => s"${model.name}::${v.name} => ${model.render(v.value)},") ++ last
        }
      }
    }

    def enumDefinition : Iterator[String] = {

      def derives = if(model.implOrd) {
        "#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]"
      } else {
        "#[derive(Copy, Clone, Debug, PartialEq, Eq)]"
      }

      def serde = {
        "#[cfg_attr(feature = \"serialization\", derive(serde::Serialize, serde::Deserialize))]"
      }


      model.comments.map(commented).iterator ++
        derives.eol ++
        serde.eol ++
        bracket(s"pub enum ${model.name}") {
          if(model.captureUnknownValues) {
            values ++ Iterator(commented("captures any value not defined in the enumeration"), "Unknown(u8),")
          } else {
            values
          }
        }
    }

    def write : Iterator[String] = {
      if(excludeWriteImpl(model)) {
        Iterator.empty
      }
      else {
        space ++
          bracket("pub(crate) fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError>") {
            s"cursor.write_u8(self.as_u8())".eol
          }
      }
    }

    def enumImpl : Iterator[String] = {
      bracket(s"impl ${model.name}") {
        (if(model.captureUnknownValues) fromValue else fromValueToOption) ++
          space ++
          asValue ++
          write
      }
    }

    enumDefinition ++ space ++ enumImpl
  }

  override def lines(implicit indentation: Indentation): Iterator[String] = {
    "use scursor::{WriteCursor, WriteError};".eol ++
    space ++
    spaced(enums.map(m => lines(m)).iterator)
  }
}
