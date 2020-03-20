package dev.gridio.dnp3.codegen.model

object EnumValue {
  def apply(name: String, value: Int, comment: String): EnumValue = EnumValue(name, value, Some(comment))
}

case class EnumValue(name: String, value: Int, comment: Option[String] = None, strName: Option[String] = None) {

  def displayName: String = strName.getOrElse(name)

}

object EnumModel {

  sealed trait Type {
    def sizeInBytes: Int
  }

  case object UInt8 extends Type {
    def sizeInBytes = 1
  }

  case object UInt16 extends Type {
    def sizeInBytes = 2
  }

  case object UInt32 extends Type {
    def sizeInBytes = 4
  }
}

sealed trait IntRender {
  def apply(i: Int): String
}

object IntRender {
  case object Hex extends IntRender {
    def apply(i: Int): String = "0x" + Integer.toHexString(i).toUpperCase
  }

  case object Base10 extends IntRender {
    def apply(i: Int): String = i.toString
  }
}

trait EnumModel {
  def name : String
  def comments: List[String]
  def values: List[EnumValue]
  def render : IntRender
  def captureUnknownValues: Boolean
}
