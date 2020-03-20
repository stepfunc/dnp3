package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.model.enums.protocol._
import dev.gridio.dnp3.codegen.render._

object ProtocolEnums extends Module {

  private def enums : List[EnumModel] = List(
    FunctionCode,
    QualifierCode,
    CommandStatus,
    OpType,
  )

  private def lines(model : EnumModel)(implicit indentation: Indentation) : Iterator[String] = {
    def values: Iterator[String] = {
      model.values.flatMap(v => v.comment.iterator.map(commented) ++ s"${v.name},".eol).iterator
    }

    def fromValueToOption: Iterator[String] = {
      bracket("pub fn from(x: u8) -> Option<Self>") {
        bracket("match x") {
          model.values.iterator.map(v => s"${model.render(v.value)} => Some(${model.name}::${v.name}),") ++ "_ => None,".eol
        }
      }
    }

    def fromValue: Iterator[String] = {
      bracket("pub fn from(x: u8) -> Self") {
        bracket("match x") {
          model.values.iterator.map(v => s"${model.render(v.value)} => ${model.name}::${v.name},") ++ s"_ => ${model.name}::Unknown(x),".eol
        }
      }
    }

    model.comments.map(commented).iterator ++
    "#[derive(Copy, Clone, Debug, PartialEq)]".eol ++
    bracket(s"pub enum ${model.name}") {
      if(model.captureUnknownValues) {
        values ++ Iterator(commented("captures any value not defined in the enumeration"), "Unknown(u8),")
      } else {
        values
      }
    } ++ space ++
    bracket(s"impl ${model.name}") {
      if(model.captureUnknownValues) fromValue else fromValueToOption
    }
  }

  override def lines(implicit indentation: Indentation): Iterator[String] = {
    spaced(enums.map(m => lines(m)).iterator)
  }
}
