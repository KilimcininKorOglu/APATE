#[inline]
pub fn read_u16_be(src: &[u8]) -> u16 {
    u16::from_be_bytes([src[0], src[1]])
}

#[inline]
pub fn read_u32_be(src: &[u8]) -> u32 {
    u32::from_be_bytes([src[0], src[1], src[2], src[3]])
}

#[inline]
pub fn read_u64_be(src: &[u8]) -> u64 {
    u64::from_be_bytes([
        src[0], src[1], src[2], src[3], src[4], src[5], src[6], src[7],
    ])
}

#[inline]
pub fn write_u16_be(dst: &mut [u8], value: u16) {
    dst[..2].copy_from_slice(&value.to_be_bytes());
}

#[inline]
pub fn write_u32_be(dst: &mut [u8], value: u32) {
    dst[..4].copy_from_slice(&value.to_be_bytes());
}

#[inline]
pub fn write_u64_be(dst: &mut [u8], value: u64) {
    dst[..8].copy_from_slice(&value.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u16_roundtrip() {
        let mut buf = [0u8; 2];
        write_u16_be(&mut buf, 0xABCD);
        assert_eq!(0xABCD, read_u16_be(&buf));
    }

    #[test]
    fn u32_roundtrip() {
        let mut buf = [0u8; 4];
        write_u32_be(&mut buf, 0xDEAD_BEEF);
        assert_eq!(0xDEAD_BEEF, read_u32_be(&buf));
    }

    #[test]
    fn u64_roundtrip() {
        let mut buf = [0u8; 8];
        write_u64_be(&mut buf, 0x0102_0304_0506_0708);
        assert_eq!(0x0102_0304_0506_0708, read_u64_be(&buf));
    }
}
