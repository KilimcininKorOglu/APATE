#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ByteCursor<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    pub fn read_u8(&mut self) -> Option<u8> {
        let byte = self.bytes.get(self.offset).copied()?;
        self.offset += 1;
        Some(byte)
    }

    pub fn read_u16_be(&mut self) -> Option<u16> {
        let bytes = self.read_exact(2)?;
        Some(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_u64_be(&mut self) -> Option<u64> {
        let bytes = self.read_exact(8)?;
        Some(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    pub fn read_exact(&mut self, len: usize) -> Option<&'a [u8]> {
        let end = self.offset.checked_add(len)?;
        let slice = self.bytes.get(self.offset..end)?;
        self.offset = end;
        Some(slice)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ByteWriter {
    bytes: Vec<u8>,
}

impl ByteWriter {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
        }
    }

    pub fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub fn write_u16_be(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_u64_be(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_bytes(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use crate::util::buf::{ByteCursor, ByteWriter};

    #[test]
    fn byte_cursor_reads_be_values() {
        let input = [0xAB, 0x12, 0x34, 0, 0, 0, 0, 0, 0, 0, 1];
        let mut cursor = ByteCursor::new(&input);

        assert_eq!(Some(0xAB), cursor.read_u8());
        assert_eq!(Some(0x1234), cursor.read_u16_be());
        assert_eq!(Some(1), cursor.read_u64_be());
        assert_eq!(0, cursor.remaining());
    }

    #[test]
    fn byte_writer_writes_be_values() {
        let mut writer = ByteWriter::with_capacity(11);
        writer.write_u8(0xAB);
        writer.write_u16_be(0x1234);
        writer.write_u64_be(1);

        assert_eq!(
            vec![0xAB, 0x12, 0x34, 0, 0, 0, 0, 0, 0, 0, 1],
            writer.into_vec()
        );
    }
}
