use crate::app::attr::{
    AnyAttribute, AttrItem, AttrProp, AttrSet, Attribute, OwnedAttribute, TypeError,
};
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

struct AttrInfo {
    prop: AttrProp,
    value: OwnedAttribute,
}

/// restricted to non-reserved attribute variations
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
struct Variation {
    value: u8,
}

impl Variation {
    fn create(value: u8) -> Result<Self, AttrError> {
        match value {
            254 | 255 => Err(AttrError::ReservedVariation(value)),
            _ => Ok(Self { value }),
        }
    }
}

/// represents a set of attributes, e.g. the default set (0)
#[derive(Default)]
pub(crate) struct SetMap {
    /// Determines if this is the default set or not
    set: AttrSet,
    inner: BTreeMap<Variation, AttrInfo>,
}

/// Errors that can occur when manipulating attributes
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum AttrError {
    /// An attribute has not been defined for this variation
    NotDefined,
    /// The attribute is already defined
    AlreadyDefined,
    /// The attribute does not match the type expected for set 0
    BadType(TypeError),
    /// The variation is reserved (254 or 255) and cannot be defined
    ReservedVariation(u8),
    /// The attribute is not writable
    NotWritable,
}

impl From<TypeError> for AttrError {
    fn from(value: TypeError) -> Self {
        Self::BadType(value)
    }
}

impl SetMap {
    /// Define an attribute in map
    ///
    /// return false if the attribute already exists
    pub(crate) fn define(&mut self, prop: AttrProp, attr: Attribute) -> Result<(), AttrError> {
        let key = Variation::create(attr.variation)?;
        // this will ensure that we're using the right types for default attributes
        let _ = AnyAttribute::try_from(&attr)?;
        match self.inner.entry(key) {
            Entry::Occupied(_) => Err(AttrError::AlreadyDefined),
            Entry::Vacant(x) => {
                x.insert(AttrInfo {
                    prop,
                    value: attr.to_owned(),
                });
                Ok(())
            }
        }
    }

    /// Write an attribute in the map
    pub(crate) fn write(&mut self, attr: Attribute) -> Result<(), AttrError> {
        let key = Variation::create(attr.variation)?;
        // this will ensure that we're using the right types for default attributes
        let _ = AnyAttribute::try_from(&attr)?;
        match self.inner.get_mut(&key) {
            None => Err(AttrError::NotDefined),
            Some(x) => {
                if x.prop.is_writable() {
                    x.value = attr.to_owned();
                    Ok(())
                } else {
                    Err(AttrError::NotWritable)
                }
            }
        }
    }

    /// Retrieve an attribute in the map
    pub(crate) fn get(&mut self, var: u8) -> Result<&OwnedAttribute, AttrError> {
        let key = Variation::create(var)?;
        match self.inner.get(&key) {
            None => Err(AttrError::NotDefined),
            Some(x) => Ok(&x.value),
        }
    }

    /// Iterate over variations
    pub(crate) fn iter(&mut self) -> impl Iterator<Item = AttrItem> + '_ {
        self.inner.iter().map(|(k, v)| AttrItem {
            variation: k.value,
            properties: v.prop,
        })
    }
}
