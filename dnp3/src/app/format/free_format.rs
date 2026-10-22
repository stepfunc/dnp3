use crate::app::format::WriteError;
use crate::app::Variation;
use scursor::WriteCursor;

pub(crate) trait FreeFormat {
    const VARIATION: Variation;

    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError>;
}
