// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.model

/*
Common variation names, written "<value format> <flags> <time>" in sentence case. Static
objects say whether they carry flags; events say whether they carry a time.
 */
object VariationNames {
  val anyVariation = "Any variation"
  val packedFormat = "Packed format"
  val withFlags = "With flags"
  val withTime = "With time"
  val withoutTime = "Without time"
  val withAbsoluteTime = "With absolute time"
  val withRelativeTime = "With relative time"
  val crob = "Control relay output block"
  val bit32 = "32-bit"
  val bit16 = "16-bit"
  val singlePrecision = "Single-precision floating point"
  val doublePrecision = "Double-precision floating point"
  val bit32WithFlags = "32-bit with flags"
  val bit16WithFlags = "16-bit with flags"
  val singlePrecisionWithFlags = "Single-precision floating point with flags"
  val doublePrecisionWithFlags = "Double-precision floating point with flags"
  val bit32WithoutFlags = "32-bit without flags"
  val bit16WithoutFlags = "16-bit without flags"
  val bit32WithFlagsAndTime = "32-bit with flags and time"
  val bit16WithFlagsAndTime = "16-bit with flags and time"
  val bit32WithFlagsAndTimeOfFreeze = "32-bit with flags and time of freeze"
  val bit16WithFlagsAndTimeOfFreeze = "16-bit with flags and time of freeze"
  val bit32WithoutTime = "32-bit without time"
  val bit16WithoutTime = "16-bit without time"
  val singlePrecisionWithoutTime = "Single-precision floating point without time"
  val doublePrecisionWithoutTime = "Double-precision floating point without time"
  val bit32WithTime = "32-bit with time"
  val bit16WithTime = "16-bit with time"
  val singlePrecisionWithTime = "Single-precision floating point with time"
  val doublePrecisionWithTime = "Double-precision floating point with time"
  val bit32WithTimeOfFreeze = "32-bit with time of freeze"
  val bit16WithTimeOfFreeze = "16-bit with time of freeze"
  val singlePrecisionWithTimeOfFreeze = "Single-precision floating point with time of freeze"
  val doublePrecisionWithTimeOfFreeze = "Double-precision floating point with time of freeze"
}
