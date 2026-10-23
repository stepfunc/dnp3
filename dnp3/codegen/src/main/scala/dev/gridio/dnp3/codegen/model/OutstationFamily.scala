// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.model

import dev.gridio.dnp3.codegen.model.groups.*

/**
 * How the Rust variation enums of a family are declared
 */
enum RustScope {
  /** `pub` enums, part of the public API */
  case Public
  /** `pub(crate)` enums */
  case Crate
  /** no generated enums: the variations are hand-written unit structs */
  case UnitStructs
}

/**
 * The no-time targets of an event variation: what an event recorded without time is reported as,
 * and for a variation with an absolute time, what an event recorded with an unsynchronized time
 * is reported as, when such a variation exists
 */
case class NoTimeTargets(withoutTime: Variation, unsynchronized: Option[Variation])

/**
 * A type of point that the outstation reports
 *
 * @param name the stem of the Rust names (`Static<name>Variation`, `Event<name>Variation`)
 * @param words the family's name in prose, for binding docs
 * @param readTarget the name of the family's `StaticTarget` / `EventTarget` variants
 * @param staticGroup the group of the static objects
 * @param eventGroup the group of the events, if the family has events
 * @param scope how the Rust variation enums are declared
 * @param ffi true if the variation enums have a bindings mirror
 * @param stored true if the database holds points of this type
 */
case class OutstationFamily(
  name: String,
  words: String,
  readTarget: String,
  staticGroup: ObjectGroup,
  eventGroup: Option[ObjectGroup],
  scope: RustScope,
  ffi: Boolean,
  stored: Boolean
) {

  // analog input deadbands are the one static family whose group is not classified as static
  require(
    staticGroup.groupType.isStatic || staticGroup.groupType == GroupType.AnalogInputDeadband,
    s"${staticGroup.name} is not a static group"
  )
  eventGroup.foreach(g => require(g.groupType.isEvent, s"${g.name} is not an event group"))
  require(!ffi || scope == RustScope.Public, s"$name has a bindings mirror but no public enums")

  /**
   * The static variations the outstation can be configured to send: every variation of the group
   * except the any-variation
   */
  def staticVariations: List[Variation] = OutstationFamily.configurable(staticGroup)

  /**
   * The event variations the outstation can be configured to send
   */
  def eventVariations: List[Variation] = eventGroup.toList.flatMap(OutstationFamily.configurable)

  /**
   * The enum names carry a static or event prefix only when the family has both
   */
  def staticEnumName: String = if (eventGroup.isDefined) s"Static${name}Variation" else s"${name}Variation"

  def eventEnumName: String = s"Event${name}Variation"

  /**
   * The no-time targets of an event variation of a stored family, derived from the fields, or
   * None if the variation has no timestamp
   */
  def noTimeTargets(v: Variation): Option[NoTimeTargets] = {
    require(eventVariations.contains(v), s"${v.name} is not an event variation of $name")
    if (!stored) None else OutstationFamily.noTimeTargets(v, eventVariations)
  }
}

object OutstationFamily {

  val all: List[OutstationFamily] = List(
    OutstationFamily("BinaryInput", "binary input", "Binary", Group1, Some(Group2), RustScope.Public, ffi = true, stored = true),
    OutstationFamily("DoubleBitBinaryInput", "double-bit binary input", "DoubleBitBinary", Group3, Some(Group4), RustScope.Public, ffi = true, stored = true),
    OutstationFamily("BinaryOutputStatus", "binary output status", "BinaryOutputStatus", Group10, Some(Group11), RustScope.Public, ffi = true, stored = true),
    OutstationFamily("Counter", "counter", "Counter", Group20, Some(Group22), RustScope.Public, ffi = true, stored = true),
    OutstationFamily("FrozenCounter", "frozen counter", "FrozenCounter", Group21, Some(Group23), RustScope.Public, ffi = true, stored = true),
    OutstationFamily("AnalogInput", "analog input", "Analog", Group30, Some(Group32), RustScope.Public, ffi = true, stored = true),
    OutstationFamily("FrozenAnalogInput", "frozen analog input", "FrozenAnalog", Group31, Some(Group33), RustScope.Public, ffi = false, stored = false),
    OutstationFamily("AnalogOutputStatus", "analog output status", "AnalogOutputStatus", Group40, Some(Group42), RustScope.Public, ffi = true, stored = true),
    OutstationFamily("AnalogInputDeadBand", "analog input deadband", "AnalogInputDeadBand", Group34, None, RustScope.Crate, ffi = false, stored = true),
    OutstationFamily("OctetString", "octet string", "OctetString", Group110, Some(Group111), RustScope.UnitStructs, ffi = false, stored = true),
  )

  /**
   * The families with generated Rust enums
   */
  def withEnums: List[OutstationFamily] = all.filter(_.scope != RustScope.UnitStructs)

  private def configurable(g: ObjectGroup): List[Variation] = g.variations.filter {
    case _: AnyVariation => false
    case _ => true
  }

  private def fields(v: Variation): List[FixedSizeField] = v match {
    case fs: FixedSize => fs.fields
    case _ => Nil
  }

  private def isTimestamp(f: FixedSizeField): Boolean = f.attr match {
    case Some(FieldAttribute.Timestamp48) | Some(FieldAttribute.Timestamp16) => true
    case _ => false
  }

  private def single(v: Variation, kind: String, matches: List[Variation]): Option[Variation] = {
    if (matches.size > 1) {
      throw new Exception(s"${v.name} has more than one ${kind} target: ${matches.map(_.name).mkString(", ")}")
    }
    matches.headOption
  }

  private[model] def noTimeTargets(v: Variation, candidates: List[Variation]): Option[NoTimeTargets] = {
    val own = fields(v)
    own.zipWithIndex.filter { case (f, _) => isTimestamp(f) } match {
      case Nil => None
      case List((time, index)) =>
        val others = candidates.filterNot(_ == v)
        val withoutTime = own.patch(index, Nil, 1)
        val target = single(v, "no-time", others.filter(c => fields(c) == withoutTime)).getOrElse {
          throw new Exception(s"${v.name} has no variation without its timestamp")
        }
        val unsynchronized = time.attr match {
          case Some(FieldAttribute.Timestamp48) =>
            def isRelative(c: Variation): Boolean = {
              val cf = fields(c)
              cf.size == own.size && cf.zip(own).zipWithIndex.forall { case ((theirs, ours), i) =>
                if (i == index) theirs.attr.contains(FieldAttribute.Timestamp16) else theirs == ours
              }
            }
            single(v, "unsynchronized", others.filter(isRelative))
          case _ => None
        }
        Some(NoTimeTargets(target, unsynchronized))
      case _ =>
        throw new Exception(s"${v.name} has more than one timestamp field")
    }
  }
}
