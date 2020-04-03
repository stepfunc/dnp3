package dev.gridio.dnp3.codegen.model

import dev.gridio.dnp3.codegen.model.enums.protocol.CommandStatus

sealed abstract class FixedSizeFieldType(val numBytes: Int)

case object UInt8Field extends FixedSizeFieldType(1)
case object UInt16Field extends FixedSizeFieldType(2)
case object UInt32Field extends FixedSizeFieldType(4)
case object SInt16Field extends FixedSizeFieldType(2)
case object SInt32Field extends FixedSizeFieldType(4)
case object Float32Field extends FixedSizeFieldType(4)
case object Float64Field extends FixedSizeFieldType(8)
case class EnumFieldType(model: EnumModel) extends FixedSizeFieldType(1)
case class CustomFieldTypeU8(structName : String) extends FixedSizeFieldType(1)
case object TimestampField extends FixedSizeFieldType(6)

sealed trait FieldType
object FieldType {
  object Flags extends FieldType;
  object Timestamp48 extends FieldType;
  object Timestamp16 extends FieldType;
  object Value extends FieldType;
}

object FixedSizeField {

  //common flags field
  val flags = FixedSizeField("flags", UInt8Field, Some(FieldType.Flags))

  // timestamps
  val time16 = FixedSizeField("time", UInt16Field, Some(FieldType.Timestamp16))
  val time48 = FixedSizeField("time", TimestampField, Some(FieldType.Timestamp48))

  // counter values
  val count16 = FixedSizeField("value", UInt16Field, Some(FieldType.Value))
  val count32 = FixedSizeField("value", UInt32Field, Some(FieldType.Value))

  // analog values
  val value16 = FixedSizeField("value", SInt16Field, Some(FieldType.Value))
  val value32 = FixedSizeField("value", SInt32Field, Some(FieldType.Value))
  val float32 = FixedSizeField("value", Float32Field, Some(FieldType.Value))
  val float64 = FixedSizeField("value", Float64Field, Some(FieldType.Value))

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

sealed case class FixedSizeField(name: String, typ: FixedSizeFieldType, attributes: Option[FieldType] = None) extends Field {

  //def isTimeUTC: Boolean = attributes.contains(FieldAttribute.IsTimeUTC)

  //def isFlags: Boolean = attributes.contains(FieldAttribute.IsFlags)

}

sealed case class VariableField(name: String) extends Field


