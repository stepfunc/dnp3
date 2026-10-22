use crate::app::parse::parser::HeaderCollection;
use crate::app::Sequence;
use crate::app::{FunctionCode, Iin2};
use crate::outstation::database::read::ReadHeader;
use crate::outstation::database::DatabaseHandle;
use crate::outstation::target::Selector;
use crate::transport::FragmentInfo;
use crate::util::bounded::{BoundedQueue, Full};
use crate::util::skipped::Skipped;

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

    pub(crate) fn set(
        &mut self,
        hash: u64,
        seq: Sequence,
        info: FragmentInfo,
        headers: HeaderCollection,
    ) {
        self.headers.clear();

        let mut iin2 = Iin2::default();
        let mut selector = Selector::new(FunctionCode::Read);
        let mut over_capacity = Skipped::new();

        for h in headers.iter() {
            let (variation, qualifier) = (h.variation(), h.qualifier());
            match ReadHeader::parse(h, &mut selector) {
                Ok(r) => {
                    if let Err(Full) = self.headers.try_push(r) {
                        over_capacity.add((variation, qualifier));
                        iin2 |= Iin2::PARAMETER_ERROR;
                    }
                }
                Err(err) => iin2 |= err,
            }
        }

        selector.log();
        if let Some(((variation, qualifier), count)) = over_capacity.first() {
            tracing::warn!(
                "insufficient capacity ({}) for READ header: {variation} - {qualifier}{count}",
                self.headers.max(),
            );
        }

        self.info = Some(DeferredInfo::new(hash, seq, info, iin2));
    }

    pub(crate) fn select(&mut self, database: &DatabaseHandle) -> Option<DeferredInfo> {
        match self.info {
            None => None,
            Some(x) => {
                let iin2 =
                    database.transaction(|db| db.inner.select_read(self.headers.iter().copied()));
                self.clear();
                Some(x.merge(iin2))
            }
        }
    }
}
