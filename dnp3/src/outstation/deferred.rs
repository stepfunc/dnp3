use crate::app::parse::parser::HeaderCollection;
use crate::app::Iin2;
use crate::app::Sequence;
use crate::outstation::database::read::ReadHeader;
use crate::outstation::database::DatabaseHandle;
use crate::transport::FragmentInfo;

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
    vec: Vec<ReadHeader>,
}

impl DeferredRead {
    pub(crate) fn new(header_capacity: u16) -> Self {
        Self {
            info: None,
            vec: Vec::with_capacity(header_capacity as usize),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.info = None;
        self.vec.clear();
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
        self.vec.clear();

        let mut iin2 = Iin2::default();

        for h in headers.iter() {
            if let Some(r) = ReadHeader::get(&h) {
                if self.vec.len() < self.vec.capacity() {
                    self.vec.push(r)
                } else {
                    tracing::warn!(
                        "insufficient capacity ({}) for READ header: {} - {}",
                        self.vec.capacity(),
                        h.variation,
                        h.details.qualifier()
                    )
                }
            } else {
                iin2 = Iin2::PARAMETER_ERROR;
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
                    for header in self.vec.iter() {
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
