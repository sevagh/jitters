use byteorder::{ByteOrder, NetworkEndian};
use rand::{thread_rng, Rng};
use std::{cmp::min, mem::size_of};

pub const JITTERS_MAX_PACKET_SIZE: usize = 1388; //some voodoo based on 1500 mtu

pub const JITTERS_SAMPLE_RATE: u32 = 44100; //we're only using 44100 L16 for now

pub struct RtpOutStream {
    flags: u16,
    sequence: u16,
    timestamp: u32,
    ssrc: u32,
}

#[repr(C, align(1))]
pub struct RtpHeader {
    flags: u16,
    sequence: u16,
    timestamp: u32,
    ssrc: u32,
}

impl RtpOutStream {
    pub fn new(channels: u16) -> Self {
        let sequence = thread_rng().gen::<u16>();
        let timestamp = thread_rng().gen::<u32>();
        let ssrc = thread_rng().gen::<u32>();

        let flags: u16 = match channels {
            1 => 0b1000000000001010,
            2 => 0b1000000000001011,
            //     VVPXCCCCMPPPPPPP
            //see: https://tools.ietf.org/html/rfc3550#section-5.1
            _ => panic!(),
        }; //PT: 10,11 for L16 44100 mono,stereo

        return RtpOutStream {
            flags,
            sequence,
            timestamp,
            ssrc,
        };
    }

    pub fn next_packet(&mut self, audio_slice: &[u8], timestamp_delta: u32) -> Vec<u8> {
        let ret_size = min(JITTERS_MAX_PACKET_SIZE, audio_slice.len());
        if ret_size > JITTERS_MAX_PACKET_SIZE {
            panic!("nah")
        }

        let hdr = self.construct_header();

        let mut ret = vec![0u8; ret_size + size_of::<RtpHeader>()];

        NetworkEndian::write_u16(&mut ret, hdr.flags);
        NetworkEndian::write_u16(&mut ret[2..], hdr.sequence);
        NetworkEndian::write_u32(&mut ret[4..], hdr.timestamp);
        NetworkEndian::write_u32(&mut ret[8..], hdr.ssrc);

        ret[size_of::<RtpHeader>()..].copy_from_slice(audio_slice);
        self.increment(timestamp_delta);
        ret
    }

    fn increment(&mut self, timestamp_delta: u32) {
        self.timestamp += timestamp_delta;
        self.sequence += 1;
    }

    fn construct_header(&self) -> RtpHeader {
        RtpHeader {
            flags: self.flags,
            ssrc: self.ssrc,
            timestamp: self.timestamp,
            sequence: self.sequence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::*;
    use memoffset::offset_of;
    use std::mem::size_of;

    #[test]
    fn test_max_packet_size() {
        // this looks like a stupid test but it should ensure that i don't blindly change
        // MAX_PACKET_SIZE without considering the implications
        assert_eq!(JITTERS_MAX_PACKET_SIZE, 1388);
    }

    #[test]
    fn test_size_of() {
        assert_eq!(size_of::<RtpHeader>(), 12);
    }

    #[test]
    fn test_offset_of() {
        assert_eq!(offset_of!(RtpHeader, flags), 0);
        assert_eq!(offset_of!(RtpHeader, sequence), 2);
        assert_eq!(offset_of!(RtpHeader, timestamp), 4);
        assert_eq!(offset_of!(RtpHeader, ssrc), 8);
    }

    #[test]
    fn test_out_stream_packets() {
        // mono channel
        let mut rtp_stream = RtpOutStream::new(1);

        let test_data_1 = vec![1u8, 3u8, 5u8, 7u8];
        let test_data_2 = vec![2u8, 4u8, 6u8, 8u8];

        let packet_1 = rtp_stream.next_packet(&test_data_1, 1);
        let packet_2 = rtp_stream.next_packet(&test_data_2, 2);

        println!("packet 1: {:#?}", packet_1);
        println!("packet 2: {:#?}", packet_2);
    }

    #[test]
    fn print_byteorder_shit() {
        let mut dest_buf: Vec<u8> = vec![0u8; 4];
        let deadbeef: u32 = 0xDEADBEEF;

        LittleEndian::write_u32(&mut dest_buf, deadbeef);
        println!("little endian: {:X?}", dest_buf);

        BigEndian::write_u32(&mut dest_buf, deadbeef);
        println!("big endian: {:X?}", dest_buf);

        NetworkEndian::write_u32(&mut dest_buf, deadbeef);
        println!("network endian: {:X?}", dest_buf);
    }
}
