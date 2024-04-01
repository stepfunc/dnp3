use crate::app::attr::{AttrDataType, AttrItem, AttrSet, AttrWriteError};
use crate::app::format::write::HeaderWriter;
use crate::app::{Iin2, QualifierCode, Variation};
use crate::outstation::database::details::attrs::map::SetMap;
use crate::outstation::database::read::AttrHeader;
use scursor::WriteCursor;
use std::collections::VecDeque;

pub(crate) mod map;

/// A single selection
#[derive(Debug)]
struct Selected {
    set: AttrSet,
    current: u8,
    end: u8,
}

impl Selected {
    fn all(set: AttrSet) -> Self {
        Self {
            set,
            current: 0,
            end: 253,
        }
    }

    fn single(set: AttrSet, var: u8) -> Self {
        Self {
            set,
            current: var,
            end: var,
        }
    }

    fn current(&self) -> (AttrSet, u8) {
        (self.set, self.current)
    }

    /// return true if completed
    fn advance(&mut self) -> bool {
        if self.current == self.end {
            true
        } else {
            match self.current.checked_add(1) {
                Some(x) => {
                    self.current = x;
                    false
                }
                None => true,
            }
        }
    }
}

struct Selection {
    max: usize,
    selected: VecDeque<Selected>,
}

fn get_list_encoding(num_items: usize) -> Option<(u8, AttrDataType)> {
    if let Some(len) = num_items.checked_mul(2) {
        // 2 bytes per attribute
        match len.try_into() {
            Ok(x) => return Some((x, AttrDataType::AttrList)),
            Err(_) => {
                if let Some(len) = len.checked_sub(256) {
                    if let Ok(x) = len.try_into() {
                        return Some((x, AttrDataType::ExtAttrList));
                    }
                }
            }
        }
    }
    None
}

impl Selection {
    fn push(&mut self, s: Selected) -> Iin2 {
        if self.selected.len() < self.max {
            self.selected.push_back(s);
            Iin2::default()
        } else {
            tracing::warn!(
                "READ exceeds max attribute headers ({}) per request",
                self.max
            );
            Iin2::PARAMETER_ERROR
        }
    }

    fn tx<F>(cursor: &mut WriteCursor, f: F) -> Result<(), scursor::WriteError>
    where
        F: FnOnce(&mut WriteCursor) -> Result<(), scursor::WriteError>,
    {
        let start = cursor.position();
        let res = f(cursor);
        if res.is_err() {
            cursor.seek_to(start)?
        }
        res
    }

    fn write_attr_list(
        set: AttrSet,
        cursor: &mut WriteCursor,
        items: impl Iterator<Item = AttrItem>,
    ) -> Result<(), scursor::WriteError> {
        if let (_, Some(num)) = items.size_hint() {
            if let Some((len, dt)) = get_list_encoding(num) {
                // we're committed to writing now
                return Self::tx(cursor, |cur| {
                    HeaderWriter::new(cur).write_range_only(
                        Variation::Group0(255),
                        set.value(),
                        set.value(),
                    )?;
                    cur.write_u8(dt.into())?;
                    cur.write_u8(len)?;
                    for item in items {
                        cur.write_u8(item.variation)?;
                        cur.write_u8(item.properties.into())?;
                    }
                    Ok(())
                });
            }
        }
        Ok(())
    }

    // return true if we wrote all selected attributes!
    fn write_all(&mut self, cursor: &mut WriteCursor, map: &SetMap) -> bool {
        while let Some(item) = self.selected.front_mut() {
            let (set, var) = item.current();

            // is it a variation list?
            if var == crate::app::attr::var::LIST_OF_ATTRIBUTE_VARIATIONS {
                if let Some(vars) = map.variations(set) {
                    if Self::write_attr_list(set, cursor, vars).is_err() {
                        return false;
                    }
                }
            } else {
                // check if it exists in the user map
                if let Ok(attr) = map.get(set, var) {
                    let mut writer = HeaderWriter::new(cursor);
                    if let Err(err) = writer.write_attribute(attr) {
                        match err {
                            AttrWriteError::Cursor => return false, // out of space
                            AttrWriteError::BadAttribute(err) => {
                                tracing::error!("Unable to write attribute: {}", err);
                            }
                        }
                    }
                }
            }

            if item.advance() {
                self.selected.pop_front();
            }
        }
        true
    }

    fn clear(&mut self) {
        self.selected.clear();
    }
}

pub(crate) struct AttrHandler {
    map: SetMap,
    selection: Selection,
}

impl AttrHandler {
    pub(crate) fn new(max_selected: usize) -> Self {
        Self {
            map: SetMap::default(),
            selection: Selection {
                max: max_selected,
                selected: VecDeque::with_capacity(max_selected),
            },
        }
    }

    pub(crate) fn get_attr_map(&mut self) -> &mut SetMap {
        &mut self.map
    }

    pub(crate) fn write(&mut self, cursor: &mut WriteCursor) -> bool {
        self.selection.write_all(cursor, &self.map)
    }

    pub(crate) fn reset(&mut self) {
        self.selection.clear();
    }

    pub(crate) fn select(&mut self, header: AttrHeader) -> Iin2 {
        match header {
            AttrHeader::All(var) => {
                match var {
                    255 => {
                        // list of variations for every set
                        let mut iin2 = Iin2::default();
                        for set in self.map.sets() {
                            iin2 |= self.selection.push(Selected::single(set, 255));
                        }
                        iin2
                    }
                    254 => {
                        // all attributes in every set
                        self.map.sets().fold(Iin2::default(), |iin, set| {
                            iin | self.selection.push(Selected::all(set))
                        })
                    }
                    _ => {
                        tracing::warn!(
                            "Attribute variation {var} may not be used with {}",
                            QualifierCode::AllObjects
                        );
                        Iin2::PARAMETER_ERROR
                    }
                }
            }
            AttrHeader::Specific(var, range) => {
                let set = match range.to_attr_set() {
                    Some(x) => x,
                    None => {
                        tracing::warn!("Attribute READ not supported with range: {range}");
                        return Iin2::PARAMETER_ERROR;
                    }
                };
                match var {
                    254 => self.selection.push(Selected::all(set)),
                    255 => self.selection.push(Selected::single(set, 255)),
                    _ => {
                        if self.map.exists(set, var) {
                            self.selection.push(Selected::single(set, var))
                        } else {
                            Iin2::NO_FUNC_CODE_SUPPORT
                        }
                    }
                }
            }
        }
    }
}
