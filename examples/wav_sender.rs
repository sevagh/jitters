use byteorder::{ByteOrder, NetworkEndian};
use hound::WavReader;
use jitters::rtp::{RtpOutStream, JITTERS_MAX_PACKET_SIZE, JITTERS_SAMPLE_RATE};
use sample::{signal, Sample, Signal};
use std::{env, net::UdpSocket, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() - 1 != 3 {
        eprintln!("usage: {} bindhostport sendhostport file.wav", args[0]);
        process::exit(-1);
    }

    let bindhostport = &args[1];
    let sendhostport = &args[2];
    let wavpath = &args[3];

    let udp_sock = UdpSocket::bind(bindhostport).unwrap();

    let reader = WavReader::open(wavpath).unwrap();
    let file_spec = reader.spec();

    if file_spec.channels > 2 {
        eprintln!("Only single or dual channel wavs supported");
        process::exit(-1);
    }

    if file_spec.sample_rate != JITTERS_SAMPLE_RATE {
        eprintln!(
            "Input clip must have sampling rate of {:#?}",
            JITTERS_SAMPLE_RATE
        );
        process::exit(-1);
    }

    let mut rtp_stream = RtpOutStream::new(file_spec.channels);
    let samples = reader
        .into_samples()
        .filter_map(Result::ok)
        .map(i16::to_sample::<f64>);
    let mut buf = vec![0u8; JITTERS_MAX_PACKET_SIZE];
    let mut time_in_ms = 0.0f64;
    let time_incr = ((1000.0 / (JITTERS_SAMPLE_RATE as f64))
        * (JITTERS_MAX_PACKET_SIZE as f64 / ((2 * file_spec.channels) as f64)))
        as f64;

    match file_spec.channels {
        1 => {
            let signal = signal::from_interleaved_samples_iter::<_, [f64; 1]>(samples);
            let chunkable_signal = signal.until_exhausted().collect::<Vec<_>>();

            let last_i = chunkable_signal.len() / (JITTERS_MAX_PACKET_SIZE / 2);

            for (i, frames) in chunkable_signal
                .chunks(JITTERS_MAX_PACKET_SIZE / 2)
                .enumerate()
            {
                for (j, frame) in frames.iter().enumerate() {
                    let single_sample = frame[0].to_sample::<i16>();
                    NetworkEndian::write_i16(&mut buf[2 * j..], single_sample);
                }
                let next_packet = if i == last_i {
                    rtp_stream.last_packet(&buf, time_in_ms as u32)
                } else {
                    rtp_stream.next_packet(&buf, time_in_ms as u32)
                };
                println!(
                    "Sent samples at timestamp {:#?}ms with RTP over UDP to {:#?}",
                    time_in_ms, sendhostport,
                );
                udp_sock.send_to(&next_packet, sendhostport).unwrap();
                time_in_ms += time_incr;
            }
        }
        2 => {
            let signal = signal::from_interleaved_samples_iter::<_, [f64; 2]>(samples);
            let chunkable_signal = signal.until_exhausted().collect::<Vec<_>>();

            let last_i = chunkable_signal.len() / (JITTERS_MAX_PACKET_SIZE / 4);

            for (i, frames) in chunkable_signal
                .chunks(JITTERS_MAX_PACKET_SIZE / 4)
                .enumerate()
            {
                for (j, frame) in frames.iter().enumerate() {
                    let chan1_sample = frame[0].to_sample::<i16>();
                    let chan2_sample = frame[1].to_sample::<i16>();
                    NetworkEndian::write_i16(&mut buf[4 * j..], chan1_sample);
                    NetworkEndian::write_i16(&mut buf[4 * j + 2..], chan2_sample); //some ratty manual interleaving
                }
                let next_packet = if i == last_i {
                    rtp_stream.last_packet(&buf, time_in_ms as u32)
                } else {
                    rtp_stream.next_packet(&buf, time_in_ms as u32)
                };
                println!(
                    "Sent samples at timestamp {:#?}ms with RTP over UDP to 127.0.0.1:1337",
                    time_in_ms
                );
                udp_sock.send_to(&next_packet, "127.0.0.1:1337").unwrap();
                time_in_ms += time_incr;
            }
        }
        _ => panic!("nah"),
    }
}
