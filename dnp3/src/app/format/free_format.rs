// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::format::WriteError;
use crate::app::Variation;
use scursor::WriteCursor;

pub(crate) trait FreeFormat {
    const VARIATION: Variation;

    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError>;
}
