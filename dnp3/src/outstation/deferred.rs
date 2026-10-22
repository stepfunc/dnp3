use crate::app::parse::parser::HeaderCollection;
use crate::app::Iin2;
use crate::app::Sequence;
use crate::outstation::database::read::ReadHeader;
use crate::outstation::database::DatabaseHandle;
use crate::transport::FragmentInfo;
use crate::util::bounded::{BoundedQueue, Full};

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

        for h in headers.iter() {
            if let Some(r) = ReadHeader::get(&h) {
                if let Err(Full) = self.headers.try_push(r) {
                    tracing::warn!(
                        "insufficient capacity ({}) for READ header: {} - {}",
                        self.headers.max(),
                        h.variation,
                        h.details.qualifier()
                    );
                    iin2 |= Iin2::PARAMETER_ERROR;
                }
            } else {
                iin2 |= ReadHeader::unsupported(h.variation);
            }
        }

        self.info = Some(DeferredInfo::new(hash, seq, info, iin2));
    }

    pub(crate) fn select(&mut self, database: &DatabaseHandle) -> Option<DeferredInfo> {
        match self.info {
            None => None,
            Some(x) => {
                let iin2 = database.transaction(|db| {
                    db.inner.reset();
                    let mut iin2 = Iin2::default();
                    for header in self.headers.iter() {
                        iin2 |= db.inner.select_by_header(*header);
                    }
                    iin2
                });
                self.clear();
                Some(x.merge(iin2))
            }
        }
    }
}
