use crate::app::file::*;
use crate::app::format::WriteError;
use crate::app::Variation;
use scursor::WriteCursor;

pub(crate) trait FreeFormat {
    const VARIATION: Variation;

    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError>;
}

impl FreeFormat for Group70Var2<'_> {
    const VARIATION: Variation = Variation::Group70Var2;

    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.write(cursor)
    }
}

impl FreeFormat for Group70Var3<'_> {
    const VARIATION: Variation = Variation::Group70Var3;

    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.write(cursor)
    }
}

impl FreeFormat for Group70Var4<'_> {
    const VARIATION: Variation = Variation::Group70Var4;

    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.write(cursor)
    }
}

impl FreeFormat for Group70Var5<'_> {
    const VARIATION: Variation = Variation::Group70Var5;

    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.write(cursor)
    }
}

impl FreeFormat for Group70Var7<'_> {
    const VARIATION: Variation = Variation::Group70Var7;

    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.write(cursor)
    }
}
