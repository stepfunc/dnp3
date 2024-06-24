package dev.gridio.dnp3.codegen.model

import dev.gridio.dnp3.codegen.model.groups._

object ObjectGroup {

  def allVariations : List[Variation] = all.flatMap(g => g.variations)

  val all: List[ObjectGroup] = List(
    Group0,
    Group1,
    Group2,
    Group3,
    Group4,
    Group10,
    Group11,
    Group12,
    Group13,
    Group20,
    Group21,
    Group22,
    Group23,
    Group30,
    Group31,
    Group32,
    Group33,
    Group34,
    Group40,
    Group41,
    Group42,
    Group43,
    Group50,
    Group51,
    Group52,
    Group60,
    Group70,
    Group80,
    Group102,
    Group110,
    Group111,
  )

}

sealed trait GroupType {
  def isStatic : Boolean = false
  def isEvent : Boolean = false
}

object GroupType {
  trait Static extends GroupType {
    override def isStatic: Boolean = true
  }
  trait Event extends GroupType {
    override def isEvent: Boolean = true
  }

  object AnalogInputDeadband extends GroupType

  object StaticBinary extends Static
  object StaticDoubleBinary extends Static
  object StaticBinaryOutputStatus extends Static
  object StaticCounter extends Static
  object StaticFrozenCounter extends Static
  object StaticAnalog extends Static
  object StaticFrozenAnalog extends Static
  object StaticAnalogOutputStatus extends Static
  object StaticOctetString extends Static

  object StaticUnsignedInteger extends Static

  object BinaryEvent extends Event
  object BinaryOutputEvent extends Event
  object BinaryOutputCommandEvent extends Event
  object DoubleBinaryEvent extends Event
  object CounterEvent extends Event
  object FrozenCounterEvent extends Event
  object AnalogEvent extends Event
  object FrozenAnalogEvent extends Event
  object AnalogOutputEvent extends Event
  object AnalogOutputCommandEvent extends Event
  object OctetStringEvent extends Event
  object VirtualTerminalEvent extends Event

  object Command extends GroupType
  object Time extends GroupType
  object ClassData extends GroupType
  object FileControl extends GroupType
  object InternalIndications extends GroupType
  object VirtualTerminalOutput extends GroupType

  object DeviceAttributes extends GroupType
}

trait ObjectGroup {

  def variations: List[Variation]

  def group: Byte

  def name: String = "Group%s".format(group)

  def desc: String

  def groupType : GroupType
}
