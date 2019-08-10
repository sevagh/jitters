const RTP_VERSION: i32 = 2;
const RTP_HEADER_LENGTH: i32 = 12;

const RTP_FLAGS_VERSION: i32 = 0xC000;
const RTP_FLAGS_PADDING: i32 = 0x2000;
const RTP_FLAGS_EXTENSION: i32 = 0x1000;
const RTP_FLAGS_CSRC_COUNT: i32 = 0x0F00;
const RTP_FLAGS_MARKER_BIT: i32 = 0x0080;
const RTP_FLAGS_PAYLOAD_TYPE: i32 = 0x007F;

const RTP_PAYLOAD_G711U: i32 = 0x00;
const RTP_PAYLOAD_GSM: i32 = 0x03;
const RTP_PAYLOAD_L16: i32 = 0x0b;
const RTP_PAYLOAD_G729A: i32 = 0x12;
const RTP_PAYLOAD_SPEEX: i32 = 0x61;
const RTP_PAYLOAD_DYNAMIC: i32 = 0x79;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
