package dev.gridio.dnp3.codegen.model

object EnumValues {

  def bitmask(list: List[String]): List[EnumValue] = from(list, Iterator.iterate(1)(i => i << 1))

  def from(list: List[String], i: Int = 0): List[EnumValue] = from(list, Iterator.from(i, 1))

  def from(list: List[String], iterator: Iterator[Int]): List[EnumValue] = list.map(s => EnumValue(s, iterator.next(), None))


  def fromPairs(list: List[(String, String)], iterator: Iterator[Int]): List[EnumValue] = list.map(x => EnumValue(x._1, iterator.next(), Some(x._2)))

  def fromPairs(list: List[(String, String)], i: Int = 0): List[EnumValue] = fromPairs(list, Iterator.from(i, 1))
}

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

  def BitFieldValues(names: List[String]): List[EnumValue] = names.zipWithIndex.map { pair =>
    EnumValue(pair._1, 1 << pair._2)
  }
}

sealed trait IntRender {
  def apply(i: Int): String
}

case object Hex extends IntRender {
  def apply(i: Int): String = "0x" + Integer.toHexString(i).toUpperCase
}

case object Base10 extends IntRender {
  def apply(i: Int): String = i.toString
}

case class EnumModel(name: String, comments: List[String], enumType: EnumModel.Type, nonDefaultValues: List[EnumValue], defaultValue: Option[EnumValue], render: IntRender = Base10) {

  def allValues: List[EnumValue] = defaultValue match {
    case Some(d) => nonDefaultValues ::: List(d)
    case None => nonDefaultValues
  }

  def default: Option[EnumValue] = defaultValue

  def defaultOrHead: EnumValue = defaultValue match {
    case Some(x) => x
    case None => nonDefaultValues.head
  }

  def qualified(ev: EnumValue): String = List(name, "::", ev.name).mkString
}
