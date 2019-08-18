#![allow(clippy::unreadable_literal, clippy::inconsistent_digit_grouping)]
// i use ugly binary digit grouping to represent the RTP header fields

use crate::rtp::RtpHeader;

pub struct RtpJitterInStream {
    first_header: RtpHeader,
    pub channels: u16,
    pub audio_slices: Vec<(Vec<u8>, u16, u32)>,
    ended: bool,
    jitter: u32,
}

impl RtpJitterInStream {
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

        RtpJitterInStream {
            first_header,
            channels,
            audio_slices,
            ended,
            jitter: 0u32,
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

        let next_seq = next_header.sequence - self.first_header.sequence; //decrement the random initial values
        let next_tstamp = next_header.timestamp - self.first_header.timestamp;

        let mut swap_idx = self.audio_slices.len() + 1;

        // check if the sequence is in order
        for (i, &(_, seq, _)) in self.audio_slices.iter().enumerate() {
            if next_seq < seq {
                swap_idx = i;
                break;
            }
        }

        self.audio_slices.push((next_audio, next_seq, next_tstamp));

        if swap_idx < self.audio_slices.len() {
            let out_of_order = self.audio_slices.swap_remove(swap_idx);
            self.audio_slices.push(out_of_order);
            self.jitter += 1;
        }

        self.ended = ((next_header.flags & 0b1_0000000) >> 7) == 0b1;
        // check the Market bit again

        /* we can do stream quality analysis later
         * by iter over audio_slices, timestamps, sequences
         * ensuring they're monotonically increasing - that's where jitter can be measured!
         */
    }

    pub fn ended(&self) -> bool {
        self.ended
    }

    pub fn jitter(&self) -> u32 {
        self.jitter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtp::*;

    #[test]
    fn test_jitter() {
        let mut rtp_out_stream = RtpOutStream::new(1);

        let test_data_1 = vec![1u8, 1u8, 1u8, 1u8];
        let test_data_2 = vec![2u8, 2u8, 2u8, 2u8];
        let test_data_3 = vec![3u8, 3u8, 3u8, 3u8];

        let packet_1 = rtp_out_stream.next_packet(&test_data_1);
        let packet_2 = rtp_out_stream.next_packet(&test_data_2);
        let packet_3 = rtp_out_stream.next_packet(&test_data_3);

        let mut rtp_in_jitter_stream = RtpJitterInStream::new(&packet_1);
        let mut rtp_in_stream = RtpInStream::new(&packet_1);

        rtp_in_jitter_stream.next_packet(&packet_3);
        rtp_in_stream.next_packet(&packet_3);

        rtp_in_jitter_stream.next_packet(&packet_2);
        rtp_in_stream.next_packet(&packet_2);

        for i in 0..4 {
            // first packet was sent in order
            assert_eq!(rtp_in_stream.audio_slices[0].0[i], 1u8);
            assert_eq!(rtp_in_jitter_stream.audio_slices[0].0[i], 1u8);

            // second was sent out of order
            assert_eq!(rtp_in_stream.audio_slices[1].0[i], 3u8);
            assert_eq!(rtp_in_jitter_stream.audio_slices[1].0[i], 2u8);

            // expect the jitter receiver to have corrected it
            assert_eq!(rtp_in_stream.audio_slices[2].0[i], 2u8);
            assert_eq!(rtp_in_jitter_stream.audio_slices[2].0[i], 3u8);
        }
    }
}
