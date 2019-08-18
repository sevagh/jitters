#![allow(clippy::unreadable_literal, clippy::inconsistent_digit_grouping)]
// i use ugly binary digit grouping to represent the RTP header fields

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

pub struct RtpInStream {
    first_header: RtpHeader,
    pub channels: u16,
    pub audio_slices: Vec<(Vec<u8>, u16, u32)>,
    ended: bool,
}

#[derive(Default, Debug)]
#[repr(C, align(1))]
pub struct RtpHeader {
    pub(crate) flags: u16,
    pub(crate) sequence: u16,
    pub(crate) timestamp: u32,
    pub(crate) ssrc: u32,
}

impl RtpOutStream {
    pub fn new(channels: u16) -> Self {
        let sequence = thread_rng().gen::<u16>() << 2; // avoid overflow for long audio clips
        let timestamp = thread_rng().gen::<u32>() << 2; // avoid overflow for long audio clips
        let ssrc = thread_rng().gen::<u32>();

        let flags: u16 = match channels {
            1 => 0b10_0_0_0000_0_0001011,
            2 => 0b10_0_0_0000_0_0001010,
            //     VV P X CCCC M PPPPPPP
            //see: https://tools.ietf.org/html/rfc3550#section-5.1
            _ => panic!(),
        }; //PT: 10,11 for L16 44100 mono,stereo

        RtpOutStream {
            flags,
            sequence,
            timestamp,
            ssrc,
        }
    }

    pub fn next_packet(&mut self, audio_slice: &[u8]) -> Vec<u8> {
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
        self.increment(audio_slice.len() as u32);
        ret
    }

    pub fn last_packet(&mut self, audio_slice: &[u8]) -> Vec<u8> {
        let ret_size = min(JITTERS_MAX_PACKET_SIZE, audio_slice.len());
        if ret_size > JITTERS_MAX_PACKET_SIZE {
            panic!("nah")
        }

        let hdr = self.construct_header();

        let mut ret = vec![0u8; ret_size + size_of::<RtpHeader>()];

        println!("End... set the market bit");
        NetworkEndian::write_u16(&mut ret, hdr.flags | 0b1_0000000); //set the Market bit
        NetworkEndian::write_u16(&mut ret[2..], hdr.sequence);
        NetworkEndian::write_u32(&mut ret[4..], hdr.timestamp);
        NetworkEndian::write_u32(&mut ret[8..], hdr.ssrc);

        ret[size_of::<RtpHeader>()..].copy_from_slice(audio_slice);
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

impl RtpInStream {
    pub fn new(first_packet: &[u8]) -> Self {
        let (first_header, first_audio) = RtpHeader::from_buf(first_packet);

        let channels: u16 = match first_header.flags & 0b1111111 {
            0b1011 => 1,
            0b1010 => 2,
            _ => panic!("unsupported payload type"),
        };

        let ended = ((first_header.flags & 0b1_0000000) >> 7) == 0b1;
        // M - marker bit is set
        // weird for a first packet...

        let mut audio_slices: Vec<(Vec<u8>, u16, u32)> = Vec::new();

        audio_slices.push((first_audio, 0u16, 0u32));

        RtpInStream {
            first_header,
            channels,
            audio_slices,
            ended,
        }
    }

    pub fn next_packet(&mut self, next_packet: &[u8]) {
        let (next_header, next_audio) = RtpHeader::from_buf(next_packet);

        if (next_header.flags & 0b11111111_0_1111111)
            != (self.first_header.flags & 0b11111111_0_1111111)
            || next_header.ssrc != self.first_header.ssrc
        {
            panic!("this packet might be from a different rtp stream")
        }

        self.audio_slices.push((
            next_audio,
            next_header.sequence - self.first_header.sequence,
            next_header.timestamp - self.first_header.timestamp,
        ));

        self.ended = ((next_header.flags & 0b1_0000000) >> 7) == 0b1;
        // check the Marker bit again

        /* we can do stream quality analysis later
         * by iter over audio_slices, timestamps, sequences
         * ensuring they're monotonically increasing - that's where jitter can be measured!
         */
    }

    pub fn ended(&self) -> bool {
        self.ended
    }
}

impl RtpHeader {
    pub fn from_buf(buf: &[u8]) -> (Self, Vec<u8>) {
        let mut rtp_header = RtpHeader::default();

        rtp_header.flags = NetworkEndian::read_u16(&buf);
        rtp_header.sequence = NetworkEndian::read_u16(&buf[2..]);
        rtp_header.timestamp = NetworkEndian::read_u32(&buf[4..]);
        rtp_header.ssrc = NetworkEndian::read_u32(&buf[8..]);

        let mut audio_slice = vec![0u8; buf.len() - size_of::<RtpHeader>()];

        if !audio_slice.is_empty() {
            audio_slice.copy_from_slice(&buf[size_of::<RtpHeader>()..]);
        }

        (rtp_header, audio_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(size_of::<RtpHeader>(), size_of::<RtpHeader>());
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

        let packet_1 = rtp_stream.next_packet(&test_data_1);
        let packet_2 = rtp_stream.next_packet(&test_data_2);

        println!("packet 1: {:#?}", packet_1);
        println!("packet 2: {:#?}", packet_2);
    }
}
