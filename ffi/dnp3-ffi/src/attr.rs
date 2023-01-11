use dnp3::app::attr::*;
use dnp3::app::Timestamp;

pub(crate) enum FfiAttrValue<'a> {
    VariationList(Option<VariationListAttr>, VariationList<'a>),
    String(Option<StringAttr>, &'a str),
    Float(Option<FloatAttr>, FloatType),
    UInt(Option<UIntAttr>, u32),
    Int(i32),
    Bool(BoolAttr, bool),
    OctetString(Option<OctetStringAttr>, &'a [u8]),
    BitString(&'a [u8]),
    DNP3Time(Option<TimeAttr>, Timestamp),
}

impl<'a> FfiAttrValue<'a> {
    pub(crate) fn extract(any: AnyAttribute<'a>) -> (AttrSet, u8, Self) {
        match any {
            AnyAttribute::Other(x) => match x.value {
                AttrValue::VisibleString(v) => (x.set, x.variation, Self::String(None, v)),
                AttrValue::UnsignedInt(v) => (x.set, x.variation, Self::UInt(None, v)),
                AttrValue::SignedInt(v) => (x.set, x.variation, Self::Int(v)),
                AttrValue::FloatingPoint(v) => (x.set, x.variation, Self::Float(None, v)),
                AttrValue::OctetString(v) => (x.set, x.variation, Self::OctetString(None, v)),
                AttrValue::Dnp3Time(v) => (x.set, x.variation, Self::DNP3Time(None, v)),
                AttrValue::BitString(v) => (x.set, x.variation, Self::BitString(v)),
                AttrValue::AttrList(v) => (x.set, x.variation, Self::VariationList(None, v)),
            },
            AnyAttribute::Known(x) => match x {
                KnownAttribute::AttributeList(e, v) => (
                    AttrSet::Default,
                    e.variation(),
                    Self::VariationList(Some(e), v),
                ),
                KnownAttribute::String(e, v) => {
                    (AttrSet::Default, e.variation(), Self::String(Some(e), v))
                }
                KnownAttribute::Float(e, v) => {
                    (AttrSet::Default, e.variation(), Self::Float(Some(e), v))
                }
                KnownAttribute::UInt(e, v) => {
                    (AttrSet::Default, e.variation(), Self::UInt(Some(e), v))
                }
                KnownAttribute::Bool(e, v) => (AttrSet::Default, e.variation(), Self::Bool(e, v)),
                KnownAttribute::OctetString(e, v) => (
                    AttrSet::Default,
                    e.variation(),
                    Self::OctetString(Some(e), v),
                ),
                KnownAttribute::DNP3Time(e, v) => {
                    (AttrSet::Default, e.variation(), Self::DNP3Time(Some(e), v))
                }
            },
        }
    }
}
