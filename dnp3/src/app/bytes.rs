#[derive(Debug, PartialEq)]
pub struct Bytes<'a> {
    pub value: &'a [u8],
}

impl<'a> Bytes<'a> {
    pub(crate) fn new(value: &'a [u8]) -> Self {
        Self { value }
    }
}

impl std::fmt::Display for Bytes<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.value.len() <= 3 {
            return write!(f, "{:02X?}", self.value);
        }

        if let Some(s) = self.value.get(0..3) {
            return write!(f, "length = {}, {:02X?} ...", self.value.len(), s);
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bytes_formats_as_expected() {
        let short = Bytes::new(&[0x01, 0x02, 0x03]);
        let long = Bytes::new(&[0x01, 0x02, 0x03, 0x04]);

        assert_eq!(format!("{}", short), "[01, 02, 03]");
        assert_eq!(format!("{}", long), "length = 4, [01, 02, 03] ...");
    }
}
