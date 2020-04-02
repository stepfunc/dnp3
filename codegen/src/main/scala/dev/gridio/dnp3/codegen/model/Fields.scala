package dev.gridio.dnp3.codegen.model

import dev.gridio.dnp3.codegen.model.enums.protocol.CommandStatus

sealed abstract class FixedSizeFieldType(val numBytes: Int)

case object UInt8Field extends FixedSizeFieldType(1)
case object UInt16Field extends FixedSizeFieldType(2)
case object UInt32Field extends FixedSizeFieldType(4)
case object UInt48Field extends FixedSizeFieldType(6)
case object SInt16Field extends FixedSizeFieldType(2)
case object SInt32Field extends FixedSizeFieldType(4)
case object Float32Field extends FixedSizeFieldType(4)
case object Float64Field extends FixedSizeFieldType(8)
case class EnumFieldType(model: EnumModel) extends FixedSizeFieldType(1)
case class CustomFieldTypeU8(structName : String) extends FixedSizeFieldType(1)
case object TimestampField extends FixedSizeFieldType(6)


object FieldAttribute extends Enumeration {
  type WeekDay = Value
  val IsTimeUTC, IsTimeRel, IsFlags = Value
}

object FixedSizeField {

  //common flags field
  val flags = FixedSizeField("flags", UInt8Field, Set(FieldAttribute.IsFlags))

  // timestamps
  val time16 = FixedSizeField("time", UInt16Field, Set(FieldAttribute.IsTimeRel))
  val time48 = FixedSizeField("time", TimestampField, Set(FieldAttribute.IsTimeUTC))

  // counter values
  val count16 = FixedSizeField("value", UInt16Field)
  val count32 = FixedSizeField("value", UInt32Field)

  // analog values
  val value16 = FixedSizeField("value", SInt16Field)
  val value32 = FixedSizeField("value", SInt32Field)
  val float32 = FixedSizeField("value", Float32Field)
  val float64 = FixedSizeField("value", Float64Field)

  //enums
  val commandStatus = FixedSizeField("status", EnumFieldType(CommandStatus))


}

object VariableFields {
  val challengeData = VariableField("challengeData")
  val hmac = VariableField("hmacValue")
  val keyWrapData = VariableField("keyWrapData")
  val errorText = VariableField("errorText")
  val certificate = VariableField("certificate")
  val userName = VariableField("userName")
  val userPublicKey = VariableField("userPublicKey")
  val certificationData = VariableField("certificationData")
  val encryptedUpdateKey = VariableField("encryptedUpdateKey")
  val signature = VariableField("digitalSignature")
}

sealed trait Field {
  def name: String
}

sealed case class FixedSizeField(name: String, typ: FixedSizeFieldType, attributes: Set[FieldAttribute.Value] = Set.empty) extends Field {

  def isTimeUTC: Boolean = attributes.contains(FieldAttribute.IsTimeUTC)

  def isFlags: Boolean = attributes.contains(FieldAttribute.IsFlags)

}

sealed case class VariableField(name: String) extends Field


