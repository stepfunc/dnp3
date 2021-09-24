pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
}

#[derive(Debug)]
pub(crate) struct EndOfStream;

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Reader {
        Reader { bytes }
    }

    pub(crate) fn clear(&mut self) {
        self.bytes = &[];
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.bytes.len()
    }

    pub(crate) fn peek_or_fail(&self) -> Result<u8, EndOfStream> {
        if self.bytes.is_empty() {
            Err(EndOfStream)
        } else {
            Ok(self.bytes[0])
        }
    }

    pub(crate) fn read_byte(&mut self) -> Result<u8, EndOfStream> {
        if self.bytes.is_empty() {
            Err(EndOfStream)
        } else {
            let value: u8 = self.bytes[0];
            self.bytes = &self.bytes[1..];
            Ok(value)
        }
    }

    pub(crate) fn take(&mut self, count: usize) -> Result<&'a [u8], EndOfStream> {
        if self.bytes.len() < count {
            Err(EndOfStream)
        } else {
            let ret = &self.bytes[0..count];
            self.bytes = &self.bytes[count..];
            Ok(ret)
        }
    }

    pub(crate) fn remainder(&self) -> &'a [u8] {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_length_on_empty_bytes_fails() {
        let mut input = Reader::new(&[]);
        assert!(input.read_byte().is_err())
    }

    #[test]
    fn consume_advances_the_input() {
        let mut input = Reader::new(&[0xCA, 0xFE]);
        assert_eq!(input.read_byte().unwrap(), 0xCA);
        assert_eq!(input.len(), 1);
        assert_eq!(input.remainder(), [0xFE]);
    }

    #[test]
    fn take_advances_the_input() {
        let mut input = Reader::new(&[0x01, 0x02, 0x03, 0x04]);

        let taken = input.take(3).unwrap();
        assert_eq!(input.len(), 1);
        assert_eq!(input.remainder(), &[0x04]);
        assert_eq!(taken.len(), 3);
        assert_eq!(taken, &[0x01, 0x02, 0x03]);
    }
}
