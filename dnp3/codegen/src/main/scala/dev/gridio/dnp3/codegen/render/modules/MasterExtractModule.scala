// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.


package dev.gridio.dnp3.codegen.render.modules

import dev.gridio.dnp3.codegen.render.*
import dev.gridio.dnp3.codegen.render.given_Conversion_String_RenderString

object MasterExtractModule extends Module {
  override def lines(implicit indent: Indentation): Iterator[String] = {
    "use crate::app::gen::prefixed::PrefixedVariation;".eol ++
    "use crate::app::gen::ranged::RangedVariation;".eol ++
    "use crate::app::measurement::Time;".eol ++
    "use crate::app::parse::traits::Index;".eol ++
    "use crate::app::{QualifierCode, Variation};".eol ++
    "use crate::master::{HeaderInfo, ReadHandler};".eol ++
    space ++
    RangedVariationModule.extractionImpl ++
    space ++
    PrefixedVariationModule.extractionImpl
  }
}
