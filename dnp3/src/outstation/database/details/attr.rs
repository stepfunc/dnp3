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

type VarMap = BTreeMap<Variation, (AttrProp, OwnedAttribute)>;

/// represents a set of attributes, e.g. the default set (0)
#[derive(Default)]
pub(crate) struct SetMap {
    sets: BTreeMap<AttrSet, VarMap>,
}

/// Errors that can occur when manipulating attributes
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum AttrError {
    /// An attribute with this value has not been defined for this variation
    AttrNotDefined(u8),
    /// No set with this value has been defined
    SetNotDefined(AttrSet),
    /// The attribute is already defined
    AlreadyDefined,
    /// The attribute does not match the type expected for set 0
    BadType(TypeError),
    /// The variation is reserved (254 or 255) and cannot be defined, written, or retrieved
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
        // this validates writable properties in the default set
        Self::validate_properties(prop, &attr)?;
        // lookup or create the set
        match self.sets.entry(attr.set) {
            Entry::Occupied(mut e) => match e.get_mut().entry(key) {
                Entry::Occupied(_) => Err(AttrError::AlreadyDefined),
                Entry::Vacant(x) => {
                    x.insert((prop, attr));
                    Ok(())
                }
            },
            Entry::Vacant(e) => {
                let mut new_set = BTreeMap::new();
                new_set.insert(key, (prop, attr));
                e.insert(new_set);
                Ok(())
            }
        }
    }

    /// Write an attribute in the map
    pub(crate) fn write(&mut self, attr: Attribute) -> Result<(), AttrError> {
        let key = Variation::create(attr.variation)?;

        match self.get_set(attr.set)?.get_mut(&key) {
            None => Err(AttrError::AttrNotDefined(key.value)),
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
    pub(crate) fn get(&mut self, set: AttrSet, var: u8) -> Result<&OwnedAttribute, AttrError> {
        let key = Variation::create(var)?;

        match self.get_set(set)?.get(&key) {
            None => Err(AttrError::AttrNotDefined(key.value)),
            Some((_, attr)) => Ok(attr),
        }
    }

    /// Iterate over variations in a set. This is useful for implementing READ on g0v254.
    pub(crate) fn set_iter(&mut self, set: AttrSet) -> Option<impl Iterator<Item = AttrItem> + '_> {
        self.sets.get(&set).map(|x| {
            x.iter().map(|(k, (prop, _))| AttrItem {
                variation: k.value,
                properties: *prop,
            })
        })
    }

    fn get_set(&mut self, set: AttrSet) -> Result<&mut VarMap, AttrError> {
        match self.sets.get_mut(&set) {
            None => Err(AttrError::SetNotDefined(set)),
            Some(set) => Ok(set),
        }
    }

    fn validate_properties(prop: AttrProp, attr: &OwnedAttribute) -> Result<(), AttrError> {
        if !prop.is_writable() {
            return Ok(());
        };

        let can_be_writable = match attr.set {
            AttrSet::Default => match attr.variation {
                203 => true, // device location altitude
                204 => true, // device location longitude
                205 => true, // device location latitude
                206 => true, // user-assigned secondary operator name
                207 => true, // user-assigned primary operator name
                208 => true, // user-assigned name
                247 => true, // user-assigned device name
                246 => true, // user-assigned ID/code
                245 => true, // user-assigned location
                240 => true, // max tx fragment size
                _ => false,
            },
            // private sets can do whatever they want
            AttrSet::Private(_) => true,
        };

        if !can_be_writable {
            return Err(AttrError::NotWritable);
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::app::attr::*;

    #[test]
    fn can_iterate_over_defined_attributes() {
        let mut map = SetMap::default();
        let attr1 = StringAttr::UserAssignedLocation.with_value("Bend");
        let attr2 = StringAttr::ConfigVersion.with_value("1.0.0");
        map.define(AttrProp::writable(), attr1).unwrap();
        map.define(AttrProp::default(), attr2).unwrap();

        // any other set will be NONE
        assert!(map.set_iter(AttrSet::new(1)).is_none());

        let mut items = map.set_iter(AttrSet::Default).unwrap();

        assert_eq!(
            items.next().unwrap(),
            AttrItem {
                variation: StringAttr::ConfigVersion.variation(),
                properties: AttrProp::default()
            }
        );
        assert_eq!(
            items.next().unwrap(),
            AttrItem {
                variation: StringAttr::UserAssignedLocation.variation(),
                properties: AttrProp::writable()
            }
        );
        assert!(items.next().is_none());
    }

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
    fn cannot_define_non_writable_attribute_as_writable() {
        let mut map = SetMap::default();
        let attr = OwnedAttribute::new(
            AttrSet::Default,
            StringAttr::DeviceSubsetAndConformance.variation(),
            OwnedAttrValue::VisibleString("3:2010".into()),
        );
        let err = map.define(AttrProp::writable(), attr).unwrap_err();
        assert_eq!(err, AttrError::NotWritable);
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
