use crate::app::attr::{
    AnyAttribute, AttrItem, AttrProp, AttrSet, Attribute, OwnedAttribute, TypeError,
};
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

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
    inner: BTreeMap<Variation, (AttrProp, OwnedAttribute)>,
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
    pub(crate) fn define(&mut self, prop: AttrProp, attr: OwnedAttribute) -> Result<(), AttrError> {
        let key = Variation::create(attr.variation)?;
        // this will ensure that we're using the right types for default attributes
        let _ = AnyAttribute::try_from(&attr.view())?;
        match self.inner.entry(key) {
            Entry::Occupied(_) => Err(AttrError::AlreadyDefined),
            Entry::Vacant(x) => {
                x.insert((prop, attr));
                Ok(())
            }
        }
    }

    /// Write an attribute in the map
    pub(crate) fn write(&mut self, attr: Attribute) -> Result<(), AttrError> {
        let key = Variation::create(attr.variation)?;
        match self.inner.get_mut(&key) {
            None => Err(AttrError::NotDefined),
            Some((prop, current)) => {
                if prop.is_writable() {
                    match attr.to_owned() {
                        None => Err(AttrError::NotWritable),
                        Some(attr) => {
                            current.value.modify(attr.value)?;
                            Ok(())
                        }
                    }
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
            Some((_, attr)) => Ok(attr),
        }
    }

    /// Iterate over variations
    pub(crate) fn iter(&mut self) -> impl Iterator<Item = AttrItem> + '_ {
        self.inner.iter().map(|(k, (prop, _))| AttrItem {
            variation: k.value,
            properties: *prop,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::app::attr::*;

    #[test]
    fn cannot_define_attributes_with_wrong_types_in_default_set() {
        let mut map = SetMap::default();
        let attr = OwnedAttribute::new(
            AttrSet::Default,
            StringAttr::UserAssignedLocation.variation(),
            OwnedAttrValue::SignedInt(42),
        );
        let err = map.define(AttrProp::writable(), attr).unwrap_err();
        assert_eq!(
            err,
            AttrError::BadType(TypeError {
                expected: AttrDataType::VisibleString,
                actual: AttrDataType::SignedInt
            })
        );
    }

    #[test]
    fn cannot_write_attribute_defined_with_different_type() {
        let mut map = SetMap::default();
        map.define(
            AttrProp::writable(),
            StringAttr::UserAssignedLocation.with_value("Bend"),
        )
        .unwrap();
        let attr = Attribute {
            set: Default::default(),
            variation: StringAttr::UserAssignedLocation.variation(),
            value: AttrValue::SignedInt(42),
        };
        let err = map.write(attr).unwrap_err();
        assert_eq!(
            err,
            AttrError::BadType(TypeError {
                expected: AttrDataType::VisibleString,
                actual: AttrDataType::SignedInt
            })
        );
    }
}
