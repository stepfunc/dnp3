// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//  _   _         ______    _ _ _   _             _ _ _
// | \ | |       |  ____|  | (_) | (_)           | | | |
// |  \| | ___   | |__   __| |_| |_ _ _ __   __ _| | | |
// | . ` |/ _ \  |  __| / _` | | __| | '_ \ / _` | | | |
// | |\  | (_) | | |___| (_| | | |_| | | | | (_| |_|_|_|
// |_| \_|\___/  |______\__,_|_|\__|_|_| |_|\__, (_|_|_)
//                                           __/ |
//                                          |___/
//
// This file is auto-generated. Do not edit manually
//

use crate::app::Variation;
use crate::outstation::database::variations::*;
use crate::outstation::database::EventClass;

use super::{EventTarget, ReadTarget, StaticTarget};

/// What a READ of a variation selects, whatever its qualifier, or `None` if a READ can't name it
pub(super) fn get(variation: Variation) -> Option<ReadTarget> {
    use AnalogInputDeadBandVariation::*;
    use EventAnalogInputVariation::*;
    use EventAnalogOutputStatusVariation::*;
    use EventBinaryInputVariation::*;
    use EventBinaryOutputStatusVariation::*;
    use EventCounterVariation::*;
    use EventDoubleBitBinaryInputVariation::*;
    use EventFrozenAnalogInputVariation::*;
    use EventFrozenCounterVariation::*;
    use ReadTarget::{Attr, Class0, Event, Static};
    use StaticAnalogInputVariation::*;
    use StaticAnalogOutputStatusVariation::*;
    use StaticBinaryInputVariation::*;
    use StaticBinaryOutputStatusVariation::*;
    use StaticCounterVariation::*;
    use StaticDoubleBitBinaryInputVariation::*;
    use StaticFrozenAnalogInputVariation::*;
    use StaticFrozenCounterVariation::*;

    let target = match variation {
        // group 0
        Variation::Group0Var254 => Attr(254),
        Variation::Group0(var) => Attr(var),
        // group 1
        Variation::Group1Var0 => Static(StaticTarget::Binary(None)),
        Variation::Group1Var1 => Static(StaticTarget::Binary(Some(Group1Var1))),
        Variation::Group1Var2 => Static(StaticTarget::Binary(Some(Group1Var2))),
        // group 2
        Variation::Group2Var0 => Event(EventTarget::Binary(None)),
        Variation::Group2Var1 => Event(EventTarget::Binary(Some(Group2Var1))),
        Variation::Group2Var2 => Event(EventTarget::Binary(Some(Group2Var2))),
        Variation::Group2Var3 => Event(EventTarget::Binary(Some(Group2Var3))),
        // group 3
        Variation::Group3Var0 => Static(StaticTarget::DoubleBitBinary(None)),
        Variation::Group3Var1 => Static(StaticTarget::DoubleBitBinary(Some(Group3Var1))),
        Variation::Group3Var2 => Static(StaticTarget::DoubleBitBinary(Some(Group3Var2))),
        // group 4
        Variation::Group4Var0 => Event(EventTarget::DoubleBitBinary(None)),
        Variation::Group4Var1 => Event(EventTarget::DoubleBitBinary(Some(Group4Var1))),
        Variation::Group4Var2 => Event(EventTarget::DoubleBitBinary(Some(Group4Var2))),
        Variation::Group4Var3 => Event(EventTarget::DoubleBitBinary(Some(Group4Var3))),
        // group 10
        Variation::Group10Var0 => Static(StaticTarget::BinaryOutputStatus(None)),
        Variation::Group10Var1 => Static(StaticTarget::BinaryOutputStatus(Some(Group10Var1))),
        Variation::Group10Var2 => Static(StaticTarget::BinaryOutputStatus(Some(Group10Var2))),
        // group 11
        Variation::Group11Var0 => Event(EventTarget::BinaryOutputStatus(None)),
        Variation::Group11Var1 => Event(EventTarget::BinaryOutputStatus(Some(Group11Var1))),
        Variation::Group11Var2 => Event(EventTarget::BinaryOutputStatus(Some(Group11Var2))),
        // group 20
        Variation::Group20Var0 => Static(StaticTarget::Counter(None)),
        Variation::Group20Var1 => Static(StaticTarget::Counter(Some(Group20Var1))),
        Variation::Group20Var2 => Static(StaticTarget::Counter(Some(Group20Var2))),
        Variation::Group20Var5 => Static(StaticTarget::Counter(Some(Group20Var5))),
        Variation::Group20Var6 => Static(StaticTarget::Counter(Some(Group20Var6))),
        // group 21
        Variation::Group21Var0 => Static(StaticTarget::FrozenCounter(None)),
        Variation::Group21Var1 => Static(StaticTarget::FrozenCounter(Some(Group21Var1))),
        Variation::Group21Var2 => Static(StaticTarget::FrozenCounter(Some(Group21Var2))),
        Variation::Group21Var5 => Static(StaticTarget::FrozenCounter(Some(Group21Var5))),
        Variation::Group21Var6 => Static(StaticTarget::FrozenCounter(Some(Group21Var6))),
        Variation::Group21Var9 => Static(StaticTarget::FrozenCounter(Some(Group21Var9))),
        Variation::Group21Var10 => Static(StaticTarget::FrozenCounter(Some(Group21Var10))),
        // group 22
        Variation::Group22Var0 => Event(EventTarget::Counter(None)),
        Variation::Group22Var1 => Event(EventTarget::Counter(Some(Group22Var1))),
        Variation::Group22Var2 => Event(EventTarget::Counter(Some(Group22Var2))),
        Variation::Group22Var5 => Event(EventTarget::Counter(Some(Group22Var5))),
        Variation::Group22Var6 => Event(EventTarget::Counter(Some(Group22Var6))),
        // group 23
        Variation::Group23Var0 => Event(EventTarget::FrozenCounter(None)),
        Variation::Group23Var1 => Event(EventTarget::FrozenCounter(Some(Group23Var1))),
        Variation::Group23Var2 => Event(EventTarget::FrozenCounter(Some(Group23Var2))),
        Variation::Group23Var5 => Event(EventTarget::FrozenCounter(Some(Group23Var5))),
        Variation::Group23Var6 => Event(EventTarget::FrozenCounter(Some(Group23Var6))),
        // group 30
        Variation::Group30Var0 => Static(StaticTarget::Analog(None)),
        Variation::Group30Var1 => Static(StaticTarget::Analog(Some(Group30Var1))),
        Variation::Group30Var2 => Static(StaticTarget::Analog(Some(Group30Var2))),
        Variation::Group30Var3 => Static(StaticTarget::Analog(Some(Group30Var3))),
        Variation::Group30Var4 => Static(StaticTarget::Analog(Some(Group30Var4))),
        Variation::Group30Var5 => Static(StaticTarget::Analog(Some(Group30Var5))),
        Variation::Group30Var6 => Static(StaticTarget::Analog(Some(Group30Var6))),
        // group 31
        Variation::Group31Var0 => Static(StaticTarget::FrozenAnalog(None)),
        Variation::Group31Var1 => Static(StaticTarget::FrozenAnalog(Some(Group31Var1))),
        Variation::Group31Var2 => Static(StaticTarget::FrozenAnalog(Some(Group31Var2))),
        Variation::Group31Var3 => Static(StaticTarget::FrozenAnalog(Some(Group31Var3))),
        Variation::Group31Var4 => Static(StaticTarget::FrozenAnalog(Some(Group31Var4))),
        Variation::Group31Var5 => Static(StaticTarget::FrozenAnalog(Some(Group31Var5))),
        Variation::Group31Var6 => Static(StaticTarget::FrozenAnalog(Some(Group31Var6))),
        Variation::Group31Var7 => Static(StaticTarget::FrozenAnalog(Some(Group31Var7))),
        Variation::Group31Var8 => Static(StaticTarget::FrozenAnalog(Some(Group31Var8))),
        // group 32
        Variation::Group32Var0 => Event(EventTarget::Analog(None)),
        Variation::Group32Var1 => Event(EventTarget::Analog(Some(Group32Var1))),
        Variation::Group32Var2 => Event(EventTarget::Analog(Some(Group32Var2))),
        Variation::Group32Var3 => Event(EventTarget::Analog(Some(Group32Var3))),
        Variation::Group32Var4 => Event(EventTarget::Analog(Some(Group32Var4))),
        Variation::Group32Var5 => Event(EventTarget::Analog(Some(Group32Var5))),
        Variation::Group32Var6 => Event(EventTarget::Analog(Some(Group32Var6))),
        Variation::Group32Var7 => Event(EventTarget::Analog(Some(Group32Var7))),
        Variation::Group32Var8 => Event(EventTarget::Analog(Some(Group32Var8))),
        // group 33
        Variation::Group33Var0 => Event(EventTarget::FrozenAnalog(None)),
        Variation::Group33Var1 => Event(EventTarget::FrozenAnalog(Some(Group33Var1))),
        Variation::Group33Var2 => Event(EventTarget::FrozenAnalog(Some(Group33Var2))),
        Variation::Group33Var3 => Event(EventTarget::FrozenAnalog(Some(Group33Var3))),
        Variation::Group33Var4 => Event(EventTarget::FrozenAnalog(Some(Group33Var4))),
        Variation::Group33Var5 => Event(EventTarget::FrozenAnalog(Some(Group33Var5))),
        Variation::Group33Var6 => Event(EventTarget::FrozenAnalog(Some(Group33Var6))),
        Variation::Group33Var7 => Event(EventTarget::FrozenAnalog(Some(Group33Var7))),
        Variation::Group33Var8 => Event(EventTarget::FrozenAnalog(Some(Group33Var8))),
        // group 34
        Variation::Group34Var0 => Static(StaticTarget::AnalogInputDeadBand(None)),
        Variation::Group34Var1 => Static(StaticTarget::AnalogInputDeadBand(Some(Group34Var1))),
        Variation::Group34Var2 => Static(StaticTarget::AnalogInputDeadBand(Some(Group34Var2))),
        Variation::Group34Var3 => Static(StaticTarget::AnalogInputDeadBand(Some(Group34Var3))),
        // group 40
        Variation::Group40Var0 => Static(StaticTarget::AnalogOutputStatus(None)),
        Variation::Group40Var1 => Static(StaticTarget::AnalogOutputStatus(Some(Group40Var1))),
        Variation::Group40Var2 => Static(StaticTarget::AnalogOutputStatus(Some(Group40Var2))),
        Variation::Group40Var3 => Static(StaticTarget::AnalogOutputStatus(Some(Group40Var3))),
        Variation::Group40Var4 => Static(StaticTarget::AnalogOutputStatus(Some(Group40Var4))),
        // group 42
        Variation::Group42Var0 => Event(EventTarget::AnalogOutputStatus(None)),
        Variation::Group42Var1 => Event(EventTarget::AnalogOutputStatus(Some(Group42Var1))),
        Variation::Group42Var2 => Event(EventTarget::AnalogOutputStatus(Some(Group42Var2))),
        Variation::Group42Var3 => Event(EventTarget::AnalogOutputStatus(Some(Group42Var3))),
        Variation::Group42Var4 => Event(EventTarget::AnalogOutputStatus(Some(Group42Var4))),
        Variation::Group42Var5 => Event(EventTarget::AnalogOutputStatus(Some(Group42Var5))),
        Variation::Group42Var6 => Event(EventTarget::AnalogOutputStatus(Some(Group42Var6))),
        Variation::Group42Var7 => Event(EventTarget::AnalogOutputStatus(Some(Group42Var7))),
        Variation::Group42Var8 => Event(EventTarget::AnalogOutputStatus(Some(Group42Var8))),
        // group 60
        Variation::Group60Var1 => Class0,
        Variation::Group60Var2 => Event(EventTarget::Class(EventClass::Class1)),
        Variation::Group60Var3 => Event(EventTarget::Class(EventClass::Class2)),
        Variation::Group60Var4 => Event(EventTarget::Class(EventClass::Class3)),
        // group 110
        Variation::Group110(0) => Static(StaticTarget::OctetString),
        // group 111
        Variation::Group111(0) => Event(EventTarget::OctetString),
        // not readable
        Variation::Group12Var1
        | Variation::Group13Var1
        | Variation::Group13Var2
        | Variation::Group41Var1
        | Variation::Group41Var2
        | Variation::Group41Var3
        | Variation::Group41Var4
        | Variation::Group43Var1
        | Variation::Group43Var2
        | Variation::Group43Var3
        | Variation::Group43Var4
        | Variation::Group43Var5
        | Variation::Group43Var6
        | Variation::Group43Var7
        | Variation::Group43Var8
        | Variation::Group50Var1
        | Variation::Group50Var2
        | Variation::Group50Var3
        | Variation::Group50Var4
        | Variation::Group51Var1
        | Variation::Group51Var2
        | Variation::Group52Var1
        | Variation::Group52Var2
        | Variation::Group70Var2
        | Variation::Group70Var3
        | Variation::Group70Var4
        | Variation::Group70Var5
        | Variation::Group70Var6
        | Variation::Group70Var7
        | Variation::Group70Var8
        | Variation::Group80Var1
        | Variation::Group102Var0
        | Variation::Group102Var1
        | Variation::Group110(_)
        | Variation::Group111(_) => return None,
    };
    Some(target)
}
