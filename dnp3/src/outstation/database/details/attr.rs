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

    /// variations in the default set *may* be configured as writable by the user
    fn can_be_written(self) -> bool {
        use crate::app::attr::var;
        std::matches!(
            self.value,
            var::DEVICE_LOCATION_ALTITUDE
                | var::DEVICE_LOCATION_LONGITUDE
                | var::DEVICE_LOCATION_LATITUDE
                | var::USER_ASSIGNED_SECONDARY_OPERATOR_NAME
                | var::USER_ASSIGNED_PRIMARY_OPERATOR_NAME
                | var::USER_ASSIGNED_OWNER_NAME
                | var::USER_ASSIGNED_DEVICE_NAME
                | var::USER_ASSIGNED_ID
                | var::USER_ASSIGNED_LOCATION
        )
    }

    /// variations in the default set that are sourced from internal configuration only
    fn is_internal(self) -> bool {
        use crate::app::attr::var;
        std::matches!(
            self.value,
            // static configuration values
            var::MAX_TX_FRAGMENT_SIZE |
                var::MAX_RX_FRAGMENT_SIZE |
                var::MAX_BINARY_OUTPUT_PER_REQUEST |
                // shape of the database - this could change between requests
                var::NUM_BINARY_INPUT |
                var::MAX_BINARY_INPUT_INDEX |
                //
                var::NUM_DOUBLE_BIT_BINARY_INPUT |
                var::MAX_DOUBLE_BIT_BINARY_INPUT_INDEX |
                //
                var::NUM_ANALOG_INPUT |
                var::MAX_ANALOG_INPUT_INDEX |
                //
                var::NUM_COUNTER |
                var::MAX_COUNTER_INDEX |
                //
                var::NUM_BINARY_OUTPUT |
                var::MAX_BINARY_OUTPUT_INDEX |
                //
                var::NUM_ANALOG_OUTPUT |
                var::MAX_ANALOG_OUTPUT_INDEX
        )
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
    /// The attribute is determined internally to the library and cannot be defined by the user
    InternalOnly,
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
        // reject reserved variations
        let variation = Variation::create(attr.variation)?;

        // ensure that we're using the right types for default attributes
        let _ = AnyAttribute::try_from(&attr.view())?;

        // constraints that apply to certain items in the default set
        if attr.set == AttrSet::Default {
            if prop.is_writable() && !variation.can_be_written() {
                return Err(AttrError::NotWritable);
            }
            if variation.is_internal() {
                return Err(AttrError::InternalOnly);
            }
        }

        // lookup or create the set
        match self.sets.entry(attr.set) {
            Entry::Occupied(mut e) => match e.get_mut().entry(variation) {
                Entry::Occupied(_) => Err(AttrError::AlreadyDefined),
                Entry::Vacant(x) => {
                    x.insert((prop, attr));
                    Ok(())
                }
            },
            Entry::Vacant(e) => {
                let mut new_set = BTreeMap::new();
                new_set.insert(variation, (prop, attr));
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
    fn cannot_define_internal_attributes_in_default_set() {
        let mut map = SetMap::default();
        let err = map
            .define(
                AttrProp::default(),
                UIntAttr::MaxBinaryOutputIndex.with_value(42),
            )
            .unwrap_err();
        assert_eq!(err, AttrError::InternalOnly);
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
