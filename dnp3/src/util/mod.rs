pub(crate) mod bit;
pub(crate) mod buffer;
pub(crate) mod channel;
pub(crate) mod cursor;
pub(crate) mod decode;
pub(crate) mod future;
pub(crate) mod phys;
pub(crate) mod slice_ext;

pub(crate) struct Smallest<T>
where
    T: Copy + PartialOrd,
{
    value: Option<T>,
}

impl<T> Smallest<T>
where
    T: Copy + PartialOrd,
{
    pub(crate) fn new() -> Self {
        Self { value: None }
    }

    pub(crate) fn value(&self) -> Option<T> {
        self.value
    }

    pub(crate) fn observe(&mut self, other: T) {
        match self.value {
            Some(previous) => {
                if other < previous {
                    self.value = Some(other);
                }
            }
            None => {
                self.value = Some(other);
            }
        }
    }
}
