package dev.gridio.dnp3.codegen.render

import dev.gridio.dnp3.codegen.model._
import dev.gridio.dnp3.codegen.render.Implicits._

object FixedSizeVariation {

  def getFieldType(f: FixedSizeFieldType) : String = {
    f match {
      case UInt8Field => "u8"
      case UInt16Field => "u16"
      case UInt32Field => "u32"
      case UInt48Field => "u64"
      case SInt16Field => "i16"
      case SInt32Field => "i32"
      case Float32Field => "f32"
      case Float64Field => "f64"
    }
  }

  def lines(gv : FixedSize)(implicit indent: Indentation): Iterator[String] = {
    s"pub struct ${gv.name} {".iter ++ indent {
      gv.fields.map(f => s"${f.name}: ${getFieldType(f.typ)},").iterator
    } ++ "}".iter
  }

}
