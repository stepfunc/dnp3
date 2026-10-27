// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::parse::parser::HeaderCollection;
use crate::app::Sequence;
use crate::app::{Iin2, QualifierCode, Variation};
use crate::outstation::database::read::{ReadHeader, ReadHeaders};
use crate::outstation::database::DatabaseHandle;
use crate::outstation::read_log::{ReadLog, Tracing};
use crate::transport::FragmentInfo;
use crate::util::bounded::{BoundedQueue, Full};
use crate::util::skipped::{Count, Skipped};

#[derive(Copy, Clone)]
pub(crate) struct DeferredInfo {
    pub(crate) hash: u64,
    pub(crate) seq: Sequence,
    pub(crate) info: FragmentInfo,
    pub(crate) iin2: Iin2,
}

impl DeferredInfo {
    fn new(hash: u64, seq: Sequence, info: FragmentInfo, iin2: Iin2) -> Self {
        DeferredInfo {
            hash,
            seq,
            info,
            iin2,
        }
    }

    fn merge(&self, iin2: Iin2) -> Self {
        Self::new(self.hash, self.seq, self.info, self.iin2 | iin2)
    }
}

pub(crate) struct DeferredRead {
    info: Option<DeferredInfo>,
    headers: BoundedQueue<ReadHeader>,
}

impl DeferredRead {
    pub(crate) fn new(max_headers: u16) -> Self {
        Self {
            info: None,
            headers: BoundedQueue::new(max_headers.into()),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.info = None;
        self.headers.clear();
    }

    pub(crate) fn is_set(&self) -> bool {
        self.info.is_some()
    }

    pub(crate) fn set<L: ReadLog>(
        &mut self,
        hash: u64,
        seq: Sequence,
        info: FragmentInfo,
        headers: HeaderCollection,
    ) {
        self.headers.clear();

        let mut iin2 = Iin2::default();
        let mut over_capacity = Skipped::new();
        let mut headers = ReadHeaders::new(&headers);

        for (header, read_with) in headers.by_ref() {
            if let Err(Full) = self.headers.try_push(header) {
                over_capacity.add(read_with);
                iin2 |= Iin2::PARAMETER_ERROR;
            }
        }

        iin2 |= headers.finish::<L>();
        if let Some((header, count)) = over_capacity.first() {
            L::deferred_capacity(self.headers.max(), header, count);
        }

        self.info = Some(DeferredInfo::new(hash, seq, info, iin2));
    }

    pub(crate) fn select(&mut self, database: &DatabaseHandle) -> Option<DeferredInfo> {
        match self.info {
            None => None,
            Some(x) => {
                let iin2 = database.transaction(|db| {
                    db.inner
                        .select_read::<Tracing>(self.headers.iter().copied())
                });
                self.clear();
                Some(x.merge(iin2))
            }
        }
    }
}

/// Log the first header of a deferred READ that doesn't fit its queue, and how many
pub(crate) fn log_capacity(
    max: usize,
    (variation, qualifier): (Variation, QualifierCode),
    count: Count,
) {
    tracing::warn!(
        "insufficient capacity ({max}) for READ header: {variation} - {qualifier}{count}"
    );
}
