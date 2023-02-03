use crate::app::{file, ObjectParseError, QualifierCode, Variation};
use scursor::ReadCursor;

#[derive(Debug)]
pub(crate) enum FreeFormatVariation<'a> {
    Group70Var2(file::Group70Var2<'a>),
    Group70Var3(file::Group70Var3<'a>),
    Group70Var4(file::Group70Var4<'a>),
    Group70Var5(file::Group70Var5<'a>),
    Group70Var6(file::Group70Var6<'a>),
    Group70Var7(file::Group70Var7<'a>),
    Group70Var8(file::Group70Var8<'a>),
}

impl<'a> FreeFormatVariation<'a> {
    pub(crate) fn parse(
        v: Variation,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<Self, ObjectParseError> {
        let object = match v {
            Variation::Group70Var2 => {
                FreeFormatVariation::Group70Var2(file::Group70Var2::read(cursor)?)
            }
            Variation::Group70Var3 => {
                FreeFormatVariation::Group70Var3(file::Group70Var3::read(cursor)?)
            }
            Variation::Group70Var4 => {
                FreeFormatVariation::Group70Var4(file::Group70Var4::read(cursor)?)
            }
            Variation::Group70Var5 => {
                FreeFormatVariation::Group70Var5(file::Group70Var5::read(cursor)?)
            }
            Variation::Group70Var6 => {
                FreeFormatVariation::Group70Var6(file::Group70Var6::read(cursor)?)
            }
            Variation::Group70Var7 => {
                FreeFormatVariation::Group70Var7(file::Group70Var7::read(cursor)?)
            }
            Variation::Group70Var8 => {
                FreeFormatVariation::Group70Var8(file::Group70Var8::read(cursor)?)
            }
            _ => {
                return Err(ObjectParseError::InvalidQualifierForVariation(
                    v,
                    QualifierCode::FreeFormat16,
                ))
            }
        };

        Ok(object)
    }

    pub(crate) fn format_objects(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FreeFormatVariation::Group70Var2(x) => x.format(f),
            FreeFormatVariation::Group70Var3(x) => x.format(f),
            FreeFormatVariation::Group70Var4(x) => x.format(f),
            FreeFormatVariation::Group70Var5(x) => x.format(f),
            FreeFormatVariation::Group70Var6(x) => x.format(f),
            FreeFormatVariation::Group70Var7(x) => x.format(f),
            FreeFormatVariation::Group70Var8(x) => x.format(f),
        }
    }
}

impl From<file::ReadError> for ObjectParseError {
    fn from(value: file::ReadError) -> Self {
        match value {
            file::ReadError::NoMoreBytes => Self::InsufficientBytes,
            file::ReadError::BadOffset { .. } => Self::BadEncoding,
            file::ReadError::Overflow => Self::BadEncoding,
            file::ReadError::BadString(_) => Self::BadEncoding,
        }
    }
}
