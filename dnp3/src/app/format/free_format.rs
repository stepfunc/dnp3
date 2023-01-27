use crate::app::file::{Group70Var2, Group70Var5};
use crate::app::format::WriteError;
use crate::app::Variation;
use scursor::WriteCursor;

pub(crate) trait FreeFormat {
    const VARIATION: Variation;

    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError>;
}

impl<'a> FreeFormat for Group70Var2<'a> {
    const VARIATION: Variation = Variation::Group70Var2;

    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.write(cursor)
    }
}

impl<'a> FreeFormat for Group70Var5<'a> {
    const VARIATION: Variation = Variation::Group70Var5;

    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.write(cursor)
    }
}
