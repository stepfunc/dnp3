// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::measurement::*;
use crate::app::variations::{Group2Var3, Group4Var3};

impl Group2Var3 {
    pub(crate) fn to_measurement(self, cto: Option<Time>) -> BinaryInput {
        let flags = Flags::new(self.flags);
        BinaryInput {
            value: flags.state(),
            flags,
            time: cto.and_then(|x| x.checked_add(self.time)),
        }
    }
}

impl Group4Var3 {
    pub(crate) fn to_measurement(self, cto: Option<Time>) -> DoubleBitBinaryInput {
        let flags = Flags::new(self.flags);
        DoubleBitBinaryInput {
            value: flags.double_bit_state(),
            flags,
            time: cto.and_then(|x| x.checked_add(self.time)),
        }
    }
}

impl From<bool> for BinaryInput {
    fn from(x: bool) -> Self {
        Self {
            value: x,
            flags: Flags::ONLINE,
            time: None,
        }
    }
}

impl From<bool> for BinaryOutputStatus {
    fn from(x: bool) -> Self {
        Self {
            value: x,
            flags: Flags::ONLINE,
            time: None,
        }
    }
}

impl From<DoubleBit> for DoubleBitBinaryInput {
    fn from(x: DoubleBit) -> Self {
        Self {
            value: x,
            flags: Flags::ONLINE,
            time: None,
        }
    }
}
