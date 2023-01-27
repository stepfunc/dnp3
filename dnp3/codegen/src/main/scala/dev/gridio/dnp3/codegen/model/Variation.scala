package dev.gridio.dnp3.codegen.model

object Variation {

  case class Value(value: Short) {
    if (value < 0 || value > 255) {
      throw new Exception(String.format("Invalid variation value: %s", value))
    }

    override def toString: java.lang.String =  {
      this.value.toString
    }
  }

  case class Id(group: Byte, variation: Variation.Value)
}



/**
 * Base trait for DNP3 objects
 */
trait Variation {


  import Variation.Id

  final def group: Byte = parent.group

  final def id: Id = Id(group, variation)

  final def name: String = s"${parent.name}Var${variation}"

  final def fullDesc: String = s"${parent.desc} - ${desc}"

  def variation: Variation.Value

  def parent: ObjectGroup

  def desc: String

  def hasFlags: Boolean = {
    this match {
      case fs: FixedSize => fs.hasFlags
      case _ => false
    }
  }
}

class FreeFormat(g: ObjectGroup, v: Byte, desc: String) extends BasicGroupVariation(g, Variation.Value(v), desc)

class AnyVariation(g: ObjectGroup, v: Byte) extends BasicGroupVariation(g, Variation.Value(v), "Any Variation")

class ClassData(g: ObjectGroup, v: Byte, desc: String) extends BasicGroupVariation(g,  Variation.Value(v), desc)

class SizedByVariation(g: ObjectGroup, v: Byte) extends BasicGroupVariation(g,  Variation.Value(v), "Sized by variation")

class SingleBitField(g: ObjectGroup, v: Byte, description: String) extends BasicGroupVariation(g,  Variation.Value(v), description)

class DoubleBitField(g: ObjectGroup, v: Byte, description: String) extends BasicGroupVariation(g,  Variation.Value(v), description)

abstract class BasicGroupVariation(g: ObjectGroup, v: Variation.Value, description: String) extends Variation {
  def variation: Variation.Value = v

  def parent: ObjectGroup = g

  def desc: String = description
}
class FixedSize(g: ObjectGroup, v: Byte, description: String)(fs: FixedSizeField*) extends BasicGroupVariation(g,  Variation.Value(v), description) {

  def hasFloatingPoint: Boolean = {
    fields.exists(f => f.isFloatingPoint)
  }

  def hasRelativeTime : Boolean = {
    fields.exists(f => f.isRelativeTime)
  }

  override def hasFlags: Boolean = {
    fields.exists(f => f.isFlags)
  }

  def fields: List[FixedSizeField] = fs.toList

  def size: Int = fs.map(x => x.typ.numBytes).sum
}
