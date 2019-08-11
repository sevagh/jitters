#[repr(C, align(1))]
struct RTPHeader {
    flags: u16,
    sequence: u16,
    timestamp: u32,
    ssrc: u32,
}

#[repr(C, align(1))]
struct RTPHeaderExt {
    profile_specific: u16,
    ext_length: u16,
    ext_data: [u32],
}

#[cfg(test)]
mod tests {
    use super::*;
    use memoffset::offset_of;
    use std::mem::size_of;

    #[test]
    fn test_size_of() {
        assert_eq!(size_of::<RTPHeader>(), 12);
    }

    #[test]
    fn test_offset_of() {
        assert_eq!(offset_of!(RTPHeader, flags), 0);
        assert_eq!(offset_of!(RTPHeader, sequence), 2);
        assert_eq!(offset_of!(RTPHeader, timestamp), 4);
        assert_eq!(offset_of!(RTPHeader, ssrc), 8);
    }
}
