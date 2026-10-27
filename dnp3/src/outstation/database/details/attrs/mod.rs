// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::attr::{
    AttrDataType, AttrItem, AttrSet, AttrWriteError, BadAttribute, OwnedAttribute,
};
use crate::app::format::write::HeaderWriter;
use crate::app::{Iin2, QualifierCode, Variation};
use crate::outstation::database::details::attrs::map::SetMap;
use crate::outstation::database::details::range::static_db::IndexRange;
use crate::outstation::database::details::WriteProgress;
use crate::outstation::database::read::AttrHeader;
use crate::outstation::read_log::ReadLog;
use crate::util::bounded::{BoundedQueue, Full};
use scursor::WriteCursor;

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
    selected: BoundedQueue<Selected>,
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
    fn push(&mut self, s: Selected, log: impl ReadLog) -> Iin2 {
        match self.selected.try_push(s) {
            Ok(()) => Iin2::default(),
            Err(Full) => {
                log.attr_selection_full(self.selected.max());
                Iin2::PARAMETER_ERROR
            }
        }
    }

    fn write_all(
        &mut self,
        cursor: &mut WriteCursor,
        map: &SetMap,
        log: impl ReadLog,
    ) -> WriteProgress {
        while let Some(item) = self.selected.front_mut() {
            let (set, var) = item.current();
            if Self::write_one(set, var, cursor, map, log) == WriteProgress::Partial {
                return WriteProgress::Partial;
            }
            if item.advance() {
                self.selected.pop_front();
            }
        }
        WriteProgress::Complete
    }

    fn write_one(
        set: AttrSet,
        var: u8,
        cursor: &mut WriteCursor,
        map: &SetMap,
        log: impl ReadLog,
    ) -> WriteProgress {
        if var == crate::app::attr::var::LIST_OF_ATTRIBUTE_VARIATIONS {
            let Some(vars) = map.variations(set) else {
                return WriteProgress::Complete;
            };
            return write_attr_list(set, cursor, vars);
        }

        let Ok(attr) = map.get(set, var) else {
            return WriteProgress::Complete;
        };
        write_attribute(cursor, attr, log)
    }

    fn clear(&mut self) {
        self.selected.clear();
    }
}

/// Run `f`, rolling the cursor back to its starting position if it fails
fn tx<E, F>(cursor: &mut WriteCursor, f: F) -> Result<(), E>
where
    E: From<scursor::WriteError>,
    F: FnOnce(&mut WriteCursor) -> Result<(), E>,
{
    let start = cursor.position();
    let res = f(cursor);
    if res.is_err() {
        cursor.seek_to(start)?
    }
    res
}

/// Write the list of attribute variations of a set (g0v255), whole or not at all
pub(crate) fn write_attr_list(
    set: AttrSet,
    cursor: &mut WriteCursor,
    items: impl ExactSizeIterator<Item = AttrItem>,
) -> WriteProgress {
    let Some((len, dt)) = get_list_encoding(items.len()) else {
        return WriteProgress::Complete;
    };
    // we're committed to writing now
    let written: Result<(), scursor::WriteError> = tx(cursor, |cur| {
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
    match written {
        Ok(()) => WriteProgress::Complete,
        Err(_) => WriteProgress::Partial,
    }
}

/// Write an attribute, whole or not at all, skipping one that can't be encoded
pub(crate) fn write_attribute(
    cursor: &mut WriteCursor,
    attr: &OwnedAttribute,
    log: impl ReadLog,
) -> WriteProgress {
    match tx(cursor, |cur| HeaderWriter::new(cur).write_attribute(attr)) {
        Ok(()) => WriteProgress::Complete,
        Err(AttrWriteError::Cursor) => WriteProgress::Partial,
        Err(AttrWriteError::BadAttribute(err)) => {
            log.attr_not_encodable(err);
            WriteProgress::Complete
        }
    }
}

pub(crate) struct AttrHandler {
    map: SetMap,
    selection: Selection,
}

impl AttrHandler {
    /// Attribute selections per READ, which `max_read_request_headers` doesn't change
    pub(crate) const MAX_SELECTED: usize = 32;

    pub(crate) fn new(max_selected: usize) -> Self {
        Self {
            map: SetMap::default(),
            selection: Selection {
                selected: BoundedQueue::new(max_selected),
            },
        }
    }

    pub(crate) fn get_attr_map(&mut self) -> &mut SetMap {
        &mut self.map
    }

    pub(crate) fn write(&mut self, cursor: &mut WriteCursor, log: impl ReadLog) -> WriteProgress {
        self.selection.write_all(cursor, &self.map, log)
    }

    pub(crate) fn reset(&mut self) {
        self.selection.clear();
    }

    pub(crate) fn select(&mut self, header: AttrHeader, log: impl ReadLog) -> Iin2 {
        match header {
            AttrHeader::All(var) => {
                match var {
                    255 => {
                        // list of variations for every set
                        let mut iin2 = Iin2::default();
                        for set in self.map.sets() {
                            iin2 |= self.selection.push(Selected::single(set, 255), log);
                        }
                        iin2
                    }
                    254 => {
                        // all attributes in every set
                        self.map.sets().fold(Iin2::default(), |iin, set| {
                            iin | self.selection.push(Selected::all(set), log)
                        })
                    }
                    _ => {
                        log.attr_variation_with_all_objects(var);
                        Iin2::PARAMETER_ERROR
                    }
                }
            }
            AttrHeader::Specific(var, range) => {
                let set = match range.to_attr_set() {
                    Some(x) => x,
                    None => {
                        log.attr_range_unsupported(range);
                        return Iin2::PARAMETER_ERROR;
                    }
                };
                match var {
                    254 => self.selection.push(Selected::all(set), log),
                    255 => self.selection.push(Selected::single(set, 255), log),
                    _ => match self.map.get(set, var) {
                        Ok(_) => self.selection.push(Selected::single(set, var), log),
                        Err(err) => err.into(),
                    },
                }
            }
        }
    }
}

pub(crate) fn log_selection_full(max: usize) {
    tracing::warn!("READ exceeds max attribute headers ({max}) per request");
}

pub(crate) fn log_variation_with_all_objects(var: u8) {
    tracing::warn!(
        "Attribute variation {var} may not be used with {}",
        QualifierCode::AllObjects
    );
}

pub(crate) fn log_range_unsupported(range: IndexRange) {
    tracing::warn!("Attribute READ not supported with range: {range}");
}

pub(crate) fn log_not_encodable(err: BadAttribute) {
    tracing::error!("Unable to write attribute: {}", err);
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::app::attr::{var, AttrProp, AttrValue, StringAttr};
    use crate::outstation::read_log::Tracing;
    use scursor::ReadCursor;

    fn encode_string(variation: u8, value: &str) -> Vec<u8> {
        let mut bytes = vec![0x00, variation, 0x00, 0x00, 0x00, 0x01];
        bytes.push(value.len().try_into().unwrap());
        bytes.extend_from_slice(value.as_bytes());
        bytes
    }

    #[test]
    fn written_attribute_lists_parse_back() {
        for count in [127u8, 128, 200, 255] {
            let items = (0..count).map(|variation| AttrItem {
                variation,
                properties: if variation % 2 == 0 {
                    AttrProp::writable()
                } else {
                    AttrProp::default()
                },
            });

            let mut buffer = [0; 520];
            let mut cursor = WriteCursor::new(&mut buffer);
            assert_eq!(
                write_attr_list(AttrSet::Private(3), &mut cursor, items.clone()),
                WriteProgress::Complete
            );

            // g0v255, 1-byte start/stop range of the set
            let (header, value) = cursor.written().split_at_checked(5).unwrap();
            assert_eq!(header, [0x00, 0xFF, 0x00, 0x03, 0x03]);

            let mut reader = ReadCursor::new(value);
            let parsed: Vec<AttrItem> = match AttrValue::parse(&mut reader).unwrap() {
                AttrValue::AttrList(x) => x.iter().collect(),
                _ => panic!("expected an attribute list of {count} items"),
            };
            assert_eq!(parsed, items.collect::<Vec<_>>());
            assert!(reader.is_empty());
        }
    }

    #[test]
    fn attribute_that_cannot_be_encoded_is_skipped_without_a_partial_header() {
        let mut handler = AttrHandler::new(1);
        let map = handler.get_attr_map();
        map.define(
            AttrProp::default(),
            StringAttr::DeviceSerialNumber.with_value("serial"),
        )
        .unwrap();
        map.define_unchecked(
            AttrProp::default(),
            StringAttr::DeviceSubsetAndConformance.with_value("L".repeat(256)),
        )
        .unwrap();
        map.define(
            AttrProp::default(),
            StringAttr::ProductNameAndModel.with_value("product"),
        )
        .unwrap();

        assert_eq!(
            handler.select(AttrHeader::All(254), Tracing),
            Iin2::default()
        );

        let mut buffer = [0; 64];
        let mut cursor = WriteCursor::new(&mut buffer);
        assert_eq!(handler.write(&mut cursor, Tracing), WriteProgress::Complete);

        let mut expected = encode_string(var::DEVICE_SERIAL_NUMBER, "serial");
        expected.extend(encode_string(var::PRODUCT_NAME_AND_MODEL, "product"));
        assert_eq!(cursor.written(), expected.as_slice());
    }
}
