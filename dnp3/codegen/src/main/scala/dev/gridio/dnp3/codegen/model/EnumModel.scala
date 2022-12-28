package dev.gridio.dnp3.codegen.model

case class EnumValue(name: String, value: Int, comment: String)

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
    def apply(i: Int): String = String.format("0x%02X", i)
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
  def implOrd: Boolean = false
}


