package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.model.EnumModel
import dev.gridio.dnp3.codegen.model.enums.protocol.FunctionCode
import dev.gridio.dnp3.codegen.render._

object ProtocolEnums extends Module {

  private def enums : List[EnumModel] = List(
      FunctionCode()
  )

  private def lines(model : EnumModel)(implicit indentation: Indentation) : Iterator[String] = {
    bracket(s"pub enum ${model.name}") {
      model.nonDefaultValues.iterator.map(v => s"${v.name},")
    } ++ space ++
    bracket(s"impl ${model.name}") {
      bracket("pub fn from(x: u8) -> Option<Self>") {
        bracket("match x") {
          model.nonDefaultValues.iterator.map(v => s"${model.render(v.value)} => Some(${model.name}::${v.name}),") ++ "_ => None,".eol
        }
      }
    }
  }

  override def lines(implicit indentation: Indentation): Iterator[String] = {
    spaced(enums.map(m => lines(m)).iterator)
  }
}
