use crate::app::attr::AttrSet;
use crate::app::{Iin2, QualifierCode};
use crate::outstation::database::details::attrs::map::SetMap;
use crate::outstation::database::read::AttrHeader;
use std::collections::VecDeque;

pub(crate) mod map;

/// models selected variations with a set
struct SetVars {
    vars: [u8; 32],
}

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

    fn advance(self) -> Option<Self> {
        if self.current == self.end {
            None
        } else {
            self.current.checked_add(1).map(|x| Self {
                set: self.set,
                current: x,
                end: self.end,
            })
        }
    }
}

struct Selection {
    max: usize,
    selected: VecDeque<Selected>,
}

impl Selection {
    fn push(&mut self, s: Selected) -> Iin2 {
        tracing::warn!("push {s:?}");

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
            map: Default::default(),
            selection: Selection {
                max: max_selected,
                selected: VecDeque::with_capacity(max_selected),
            },
        }
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
