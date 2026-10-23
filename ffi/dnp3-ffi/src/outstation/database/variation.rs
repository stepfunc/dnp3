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

use dnp3::outstation::database::*;

use crate::ffi;

impl From<ffi::StaticBinaryInputVariation> for StaticBinaryInputVariation {
    fn from(from: ffi::StaticBinaryInputVariation) -> Self {
        match from {
            ffi::StaticBinaryInputVariation::Group1Var1 => Self::Group1Var1,
            ffi::StaticBinaryInputVariation::Group1Var2 => Self::Group1Var2,
        }
    }
}

impl From<ffi::EventBinaryInputVariation> for EventBinaryInputVariation {
    fn from(from: ffi::EventBinaryInputVariation) -> Self {
        match from {
            ffi::EventBinaryInputVariation::Group2Var1 => Self::Group2Var1,
            ffi::EventBinaryInputVariation::Group2Var2 => Self::Group2Var2,
            ffi::EventBinaryInputVariation::Group2Var3 => Self::Group2Var3,
        }
    }
}

impl From<ffi::StaticDoubleBitBinaryInputVariation> for StaticDoubleBitBinaryInputVariation {
    fn from(from: ffi::StaticDoubleBitBinaryInputVariation) -> Self {
        match from {
            ffi::StaticDoubleBitBinaryInputVariation::Group3Var1 => Self::Group3Var1,
            ffi::StaticDoubleBitBinaryInputVariation::Group3Var2 => Self::Group3Var2,
        }
    }
}

impl From<ffi::EventDoubleBitBinaryInputVariation> for EventDoubleBitBinaryInputVariation {
    fn from(from: ffi::EventDoubleBitBinaryInputVariation) -> Self {
        match from {
            ffi::EventDoubleBitBinaryInputVariation::Group4Var1 => Self::Group4Var1,
            ffi::EventDoubleBitBinaryInputVariation::Group4Var2 => Self::Group4Var2,
            ffi::EventDoubleBitBinaryInputVariation::Group4Var3 => Self::Group4Var3,
        }
    }
}

impl From<ffi::StaticBinaryOutputStatusVariation> for StaticBinaryOutputStatusVariation {
    fn from(from: ffi::StaticBinaryOutputStatusVariation) -> Self {
        match from {
            ffi::StaticBinaryOutputStatusVariation::Group10Var1 => Self::Group10Var1,
            ffi::StaticBinaryOutputStatusVariation::Group10Var2 => Self::Group10Var2,
        }
    }
}

impl From<ffi::EventBinaryOutputStatusVariation> for EventBinaryOutputStatusVariation {
    fn from(from: ffi::EventBinaryOutputStatusVariation) -> Self {
        match from {
            ffi::EventBinaryOutputStatusVariation::Group11Var1 => Self::Group11Var1,
            ffi::EventBinaryOutputStatusVariation::Group11Var2 => Self::Group11Var2,
        }
    }
}

impl From<ffi::StaticCounterVariation> for StaticCounterVariation {
    fn from(from: ffi::StaticCounterVariation) -> Self {
        match from {
            ffi::StaticCounterVariation::Group20Var1 => Self::Group20Var1,
            ffi::StaticCounterVariation::Group20Var2 => Self::Group20Var2,
            ffi::StaticCounterVariation::Group20Var5 => Self::Group20Var5,
            ffi::StaticCounterVariation::Group20Var6 => Self::Group20Var6,
        }
    }
}

impl From<ffi::EventCounterVariation> for EventCounterVariation {
    fn from(from: ffi::EventCounterVariation) -> Self {
        match from {
            ffi::EventCounterVariation::Group22Var1 => Self::Group22Var1,
            ffi::EventCounterVariation::Group22Var2 => Self::Group22Var2,
            ffi::EventCounterVariation::Group22Var5 => Self::Group22Var5,
            ffi::EventCounterVariation::Group22Var6 => Self::Group22Var6,
        }
    }
}

impl From<ffi::StaticFrozenCounterVariation> for StaticFrozenCounterVariation {
    fn from(from: ffi::StaticFrozenCounterVariation) -> Self {
        match from {
            ffi::StaticFrozenCounterVariation::Group21Var1 => Self::Group21Var1,
            ffi::StaticFrozenCounterVariation::Group21Var2 => Self::Group21Var2,
            ffi::StaticFrozenCounterVariation::Group21Var5 => Self::Group21Var5,
            ffi::StaticFrozenCounterVariation::Group21Var6 => Self::Group21Var6,
            ffi::StaticFrozenCounterVariation::Group21Var9 => Self::Group21Var9,
            ffi::StaticFrozenCounterVariation::Group21Var10 => Self::Group21Var10,
        }
    }
}

impl From<ffi::EventFrozenCounterVariation> for EventFrozenCounterVariation {
    fn from(from: ffi::EventFrozenCounterVariation) -> Self {
        match from {
            ffi::EventFrozenCounterVariation::Group23Var1 => Self::Group23Var1,
            ffi::EventFrozenCounterVariation::Group23Var2 => Self::Group23Var2,
            ffi::EventFrozenCounterVariation::Group23Var5 => Self::Group23Var5,
            ffi::EventFrozenCounterVariation::Group23Var6 => Self::Group23Var6,
        }
    }
}

impl From<ffi::StaticAnalogInputVariation> for StaticAnalogInputVariation {
    fn from(from: ffi::StaticAnalogInputVariation) -> Self {
        match from {
            ffi::StaticAnalogInputVariation::Group30Var1 => Self::Group30Var1,
            ffi::StaticAnalogInputVariation::Group30Var2 => Self::Group30Var2,
            ffi::StaticAnalogInputVariation::Group30Var3 => Self::Group30Var3,
            ffi::StaticAnalogInputVariation::Group30Var4 => Self::Group30Var4,
            ffi::StaticAnalogInputVariation::Group30Var5 => Self::Group30Var5,
            ffi::StaticAnalogInputVariation::Group30Var6 => Self::Group30Var6,
        }
    }
}

impl From<ffi::EventAnalogInputVariation> for EventAnalogInputVariation {
    fn from(from: ffi::EventAnalogInputVariation) -> Self {
        match from {
            ffi::EventAnalogInputVariation::Group32Var1 => Self::Group32Var1,
            ffi::EventAnalogInputVariation::Group32Var2 => Self::Group32Var2,
            ffi::EventAnalogInputVariation::Group32Var3 => Self::Group32Var3,
            ffi::EventAnalogInputVariation::Group32Var4 => Self::Group32Var4,
            ffi::EventAnalogInputVariation::Group32Var5 => Self::Group32Var5,
            ffi::EventAnalogInputVariation::Group32Var6 => Self::Group32Var6,
            ffi::EventAnalogInputVariation::Group32Var7 => Self::Group32Var7,
            ffi::EventAnalogInputVariation::Group32Var8 => Self::Group32Var8,
        }
    }
}

impl From<ffi::StaticAnalogOutputStatusVariation> for StaticAnalogOutputStatusVariation {
    fn from(from: ffi::StaticAnalogOutputStatusVariation) -> Self {
        match from {
            ffi::StaticAnalogOutputStatusVariation::Group40Var1 => Self::Group40Var1,
            ffi::StaticAnalogOutputStatusVariation::Group40Var2 => Self::Group40Var2,
            ffi::StaticAnalogOutputStatusVariation::Group40Var3 => Self::Group40Var3,
            ffi::StaticAnalogOutputStatusVariation::Group40Var4 => Self::Group40Var4,
        }
    }
}

impl From<ffi::EventAnalogOutputStatusVariation> for EventAnalogOutputStatusVariation {
    fn from(from: ffi::EventAnalogOutputStatusVariation) -> Self {
        match from {
            ffi::EventAnalogOutputStatusVariation::Group42Var1 => Self::Group42Var1,
            ffi::EventAnalogOutputStatusVariation::Group42Var2 => Self::Group42Var2,
            ffi::EventAnalogOutputStatusVariation::Group42Var3 => Self::Group42Var3,
            ffi::EventAnalogOutputStatusVariation::Group42Var4 => Self::Group42Var4,
            ffi::EventAnalogOutputStatusVariation::Group42Var5 => Self::Group42Var5,
            ffi::EventAnalogOutputStatusVariation::Group42Var6 => Self::Group42Var6,
            ffi::EventAnalogOutputStatusVariation::Group42Var7 => Self::Group42Var7,
            ffi::EventAnalogOutputStatusVariation::Group42Var8 => Self::Group42Var8,
        }
    }
}
