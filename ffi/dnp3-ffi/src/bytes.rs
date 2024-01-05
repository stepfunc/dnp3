pub struct ByteCollection {
    pub(crate) inner: Vec<u8>,
}

pub(crate) fn byte_collection_create(reserve_size: u32) -> *mut ByteCollection {
    let bytes = Box::new(ByteCollection {
        inner: Vec::with_capacity(reserve_size as usize),
    });
    Box::into_raw(bytes)
}

pub(crate) unsafe fn byte_collection_destroy(bytes: *mut ByteCollection) {
    if !bytes.is_null() {
        drop(Box::from_raw(bytes));
    }
}

pub(crate) unsafe fn byte_collection_add(instance: *mut ByteCollection, value: u8) {
    if let Some(instance) = instance.as_mut() {
        instance.inner.push(value)
    }
}

pub struct ByteIterator<'a> {
    inner: std::slice::Iter<'a, u8>,
    next: Option<u8>,
}

impl<'a> ByteIterator<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self {
            inner: bytes.iter(),
            next: None,
        }
    }

    fn next(&mut self) {
        self.next = self.inner.next().copied()
    }
}

pub unsafe fn byte_iterator_next(it: *mut ByteIterator) -> *const u8 {
    let it = it.as_mut();
    match it {
        None => std::ptr::null(),
        Some(x) => {
            x.next();
            match &x.next {
                None => std::ptr::null(),
                Some(x) => x,
            }
        }
    }
}
