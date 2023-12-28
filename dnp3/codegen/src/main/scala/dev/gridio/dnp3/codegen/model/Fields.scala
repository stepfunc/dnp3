package dev.gridio.dnp3.codegen.model

import dev.gridio.dnp3.codegen.model.enums.protocol.CommandStatus

sealed abstract class FixedSizeFieldType(val numBytes: Int)

case object UInt8Field extends FixedSizeFieldType(1)
case object UInt16Field extends FixedSizeFieldType(2)
case object UInt32Field extends FixedSizeFieldType(4)
case object S16Field extends FixedSizeFieldType(2)
case object S32Field extends FixedSizeFieldType(4)
case object Float32Field extends FixedSizeFieldType(4)
case object Float64Field extends FixedSizeFieldType(8)
case class EnumFieldType(model: EnumModel) extends FixedSizeFieldType(1)
case class CustomFieldTypeU8(structName : String) extends FixedSizeFieldType(1)
case object TimestampField extends FixedSizeFieldType(6)

sealed trait FieldAttribute
object FieldAttribute {
  object Flags extends FieldAttribute;
  object Timestamp48 extends FieldAttribute;
  object Timestamp16 extends FieldAttribute;
  object Value extends FieldAttribute;
}

object FixedSizeField {

  //common flags field
  val flags = FixedSizeField("flags", UInt8Field, Some(FieldAttribute.Flags))

  // timestamps
  val time16 = FixedSizeField("time", UInt16Field, Some(FieldAttribute.Timestamp16))
  val time48 = FixedSizeField("time", TimestampField, Some(FieldAttribute.Timestamp48))

  // counter values
  val count16 = FixedSizeField("value", UInt16Field, Some(FieldAttribute.Value))
  val count32 = FixedSizeField("value", UInt32Field, Some(FieldAttribute.Value))

  // analog values
  val value16 = FixedSizeField("value", S16Field, Some(FieldAttribute.Value))
  val value32 = FixedSizeField("value", S32Field, Some(FieldAttribute.Value))
  val float32 = FixedSizeField("value", Float32Field, Some(FieldAttribute.Value))
  val float64 = FixedSizeField("value", Float64Field, Some(FieldAttribute.Value))

  // unsigned values
  val valueU16 = FixedSizeField("value", UInt16Field, Some(FieldAttribute.Value))
  val valueU32 = FixedSizeField("value", UInt32Field, Some(FieldAttribute.Value))

  //enums
  val commandStatus = FixedSizeField("status", EnumFieldType(CommandStatus))

  // 8-bit integer
  val unsignedByte = FixedSizeField("value", UInt8Field, Some(FieldAttribute.Value))
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

sealed case class FixedSizeField(name: String, typ: FixedSizeFieldType, attr: Option[FieldAttribute] = None) extends Field {
  def isRelativeTime : Boolean = {
    attr match {
      case Some(FieldAttribute.Timestamp16) => true
      case _ => false
    }
  }

  def isFloatingPoint: Boolean = {
    typ match {
      case Float32Field  => true
      case Float64Field => true
      case _ => false
    }
  }

  def isFlags : Boolean = {
    attr match {
      case Some(FieldAttribute.Flags) => true
      case _ => false
    }
  }

  def isValue : Boolean = {
    attr match {
      case Some(FieldAttribute.Value) => true
      case _ => false
    }
  }
}

sealed case class VariableField(name: String) extends Field


