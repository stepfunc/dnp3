use crate::app::attr::{
    AnyAttribute, AttrDataType, AttrItem, AttrProp, AttrSet, Attribute, OwnedAttribute, TypeError,
};
use crate::outstation::database::AttrDefError;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

/// restricted to non-reserved attribute variations
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
struct Variation {
    value: u8,
}

struct Reserved(u8);

impl From<Reserved> for AttrDefError {
    fn from(value: Reserved) -> Self {
        Self::ReservedVariation(value.0)
    }
}

impl From<Reserved> for AttrError {
    fn from(value: Reserved) -> Self {
        Self::ReservedVariation(value.0)
    }
}

impl Variation {
    fn create(value: u8) -> Result<Self, Reserved> {
        match value {
            0 | 254 | 255 => Err(Reserved(value)),
            _ => Ok(Self { value }),
        }
    }

    /// variations in the default set *may* be configured as writable by the user
    fn can_be_written(self) -> bool {
        use crate::app::attr::var;
        std::matches!(
            self.value,
            var::USER_ASSIGNED_SECONDARY_OPERATOR_NAME
                | var::USER_ASSIGNED_PRIMARY_OPERATOR_NAME
                | var::USER_ASSIGNED_OWNER_NAME
                | var::USER_ASSIGNED_DEVICE_NAME
                | var::USER_ASSIGNED_ID
                | var::USER_ASSIGNED_LOCATION
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
    AttrNotDefined(AttrSet, u8),
    /// No set with this value has been defined
    SetNotDefined(AttrSet),
    /// The attribute does not match the type expected for set 0
    BadType(TypeError),
    /// The variation is reserved (254 or 255) and cannot be defined, written, or retrieved
    ReservedVariation(u8),
    /// The attribute is not writable
    NotWritable(AttrSet, u8),
}

impl std::fmt::Display for AttrError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::AttrNotDefined(set, x) => write!(
                f,
                "Attribute with set = {set:?} is not defined for variation {x}"
            ),
            Self::SetNotDefined(x) => write!(f, "Attribute set not defined: {x:?}"),
            Self::BadType(x) => write!(
                f,
                "The type {:?} does not match the expected type {:?}",
                x.actual, x.expected
            ),
            Self::ReservedVariation(x) => {
                write!(f, "Reserved variation cannot be defined or written: {x}")
            }
            Self::NotWritable(set, var) => write!(
                f,
                "Attribute with set = {set:?} and var = {var} cannot be written"
            ),
        }
    }
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
    pub(crate) fn define(
        &mut self,
        prop: AttrProp,
        attr: OwnedAttribute,
    ) -> Result<(), AttrDefError> {
        // reject reserved variations
        let variation = Variation::create(attr.variation)?;

        // ensure that we're using the right types for default attributes
        let _ = AnyAttribute::try_from(&attr.view())?;

        // constraints that apply to certain items in the default set
        if attr.set == AttrSet::Default && prop.is_writable() && !variation.can_be_written() {
            return Err(AttrDefError::NotWritable(attr.set, attr.variation));
        }

        // lookup or create the set
        match self.sets.entry(attr.set) {
            Entry::Occupied(mut e) => match e.get_mut().entry(variation) {
                Entry::Occupied(_) => Err(AttrDefError::AlreadyDefined),
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

    fn same_type(expected: AttrDataType, actual: AttrDataType) -> Result<(), TypeError> {
        if expected == actual {
            Ok(())
        } else {
            Err(TypeError::new(expected, actual))
        }
    }

    /// Validate that the write would succeed
    pub(crate) fn can_write(&mut self, attr: Attribute) -> Result<(), AttrError> {
        self.maybe_write(attr, false)
    }

    /// Write an attribute in the map
    pub(crate) fn write(&mut self, attr: Attribute) -> Result<(), AttrError> {
        self.maybe_write(attr, true)
    }

    fn maybe_write(&mut self, attr: Attribute, commit: bool) -> Result<(), AttrError> {
        let key = Variation::create(attr.variation)?;
        match self.get_set_mut(attr.set)?.get_mut(&key) {
            None => Err(AttrError::AttrNotDefined(attr.set, key.value)),
            Some((prop, current)) => {
                if prop.is_writable() {
                    Self::same_type(current.value.data_type(), attr.value.data_type())?;
                    if commit {
                        match attr.to_owned() {
                            None => Err(AttrError::NotWritable(attr.set, key.value)),
                            Some(mut attr) => {
                                std::mem::swap(&mut attr, current);
                                Ok(())
                            }
                        }
                    } else {
                        Ok(())
                    }
                } else {
                    Err(AttrError::NotWritable(attr.set, key.value))
                }
            }
        }
    }

    pub(crate) fn exists(&self, set: AttrSet, var: u8) -> bool {
        self.get(set, var).is_ok()
    }

    /// Retrieve an attribute in the map
    pub(crate) fn get(&self, set: AttrSet, var: u8) -> Result<&OwnedAttribute, AttrError> {
        let key = Variation::create(var)?;
        match self.get_set(set)?.get(&key) {
            None => Err(AttrError::AttrNotDefined(set, key.value)),
            Some((_, attr)) => Ok(attr),
        }
    }

    /// Iterate over variations in a requested set. This is useful for implementing READ on g0v254.
    pub(crate) fn variations(&self, set: AttrSet) -> Option<impl Iterator<Item = AttrItem> + '_> {
        self.sets.get(&set).map(|x| {
            x.iter().map(|(k, (prop, _))| AttrItem {
                variation: k.value,
                properties: *prop,
            })
        })
    }

    /// Iterate over all the sets. This is useful for READ 255 w/ 0x06.
    pub(crate) fn sets(&self) -> impl Iterator<Item = AttrSet> + '_ {
        self.sets.keys().copied()
    }

    fn get_set_mut(&mut self, set: AttrSet) -> Result<&mut VarMap, AttrError> {
        match self.sets.get_mut(&set) {
            None => Err(AttrError::SetNotDefined(set)),
            Some(set) => Ok(set),
        }
    }

    fn get_set(&self, set: AttrSet) -> Result<&VarMap, AttrError> {
        match self.sets.get(&set) {
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
        assert!(map.variations(AttrSet::new(1)).is_none());

        let mut items = map.variations(AttrSet::Default).unwrap();

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
            AttrDefError::BadType(TypeError {
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
        assert_eq!(
            err,
            AttrDefError::NotWritable(
                AttrSet::Default,
                StringAttr::DeviceSubsetAndConformance.variation()
            )
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
