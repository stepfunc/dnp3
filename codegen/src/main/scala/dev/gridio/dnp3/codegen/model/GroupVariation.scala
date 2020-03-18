package dev.gridio.dnp3.codegen.model

object GroupVariation {

  case class Id(group: Byte, variation: Byte)

}


/**
 * Base trait for DNP3 objects
 */
trait GroupVariation {

  import GroupVariation.Id

  final def group: Byte = parent.group

  final def id: Id = Id(group, variation)

  final def name: String = "%sVar%s".format(parent.name, variation)

  final def fullDesc: String = "%s - %s".format(parent.desc, desc)

  final def shortValue: Int = group * 256 + variation

  def variation: Byte

  def parent: ObjectGroup

  def desc: String

  def attributes: Set[FieldAttribute.Value]
}

class AnyVariation(g: ObjectGroup, v: Byte) extends BasicGroupVariation(g, v, "Any Variation")

class ClassData(g: ObjectGroup, v: Byte, desc: String) extends BasicGroupVariation(g, v, desc)

class SizedByVariation(g: ObjectGroup, v: Byte) extends BasicGroupVariation(g, v, "Sized by variation")

abstract class DefaultVariableSize(g: ObjectGroup, v: Byte, description: String) extends BasicGroupVariation(g, v, description)

class SingleBitfield(g: ObjectGroup, v: Byte, description: String) extends BasicGroupVariation(g, v, description)

class DoubleBitfield(g: ObjectGroup, v: Byte, description: String) extends BasicGroupVariation(g, v, description)

sealed abstract class BasicGroupVariation(g: ObjectGroup, v: Byte, description: String) extends GroupVariation {
  def variation: Byte = v

  def parent: ObjectGroup = g

  def desc: String = description

  def attributes: Set[FieldAttribute.Value] = Set.empty
}

class AuthVariableSize(g: ObjectGroup,
                       v: Byte,
                       description: String,
                       val fixedFields: List[FixedSizeField],
                       val lengthFields: List[VariableField],
                       val remainder: Option[VariableField]) extends BasicGroupVariation(g, v, description) {

  /// The total minimum size for the aggregate object
  def minimumSize: Int = {

    def fixedSize = fixedFields.map(x => x.typ.numBytes).sum

    def variableSize = 2 * lengthFields.length

    fixedSize + variableSize
  }

}

class RemainderOnly(g: ObjectGroup, v: Byte, description: String, remainder: VariableField) extends AuthVariableSize(g, v, description, Nil, Nil, Some(remainder)) {
  def remainderValue: VariableField = remainder
}

class FixedSize(g: ObjectGroup, v: Byte, description: String)(fs: FixedSizeField*) extends BasicGroupVariation(g, v, description) {

  def fields: List[FixedSizeField] = fs.toList

  def size: Int = fs.map(x => x.typ.numBytes).sum

  override def attributes: Set[FieldAttribute.Value] = fs.flatMap(_.attributes).toSet
}
