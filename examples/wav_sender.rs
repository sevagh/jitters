use hound::WavReader;
use byteorder::{ByteOrder, NetworkEndian};
use jitters::{RtpOutStream, JITTERS_MAX_PACKET_SIZE, JITTERS_SAMPLE_RATE};
use sample::{signal, Signal, Sample};
use std::{env, net::UdpSocket, process};

fn main() {
    let udp_sock = UdpSocket::bind("127.0.0.1:13337").unwrap();

    let args: Vec<String> = env::args().collect();
    if args.len() - 1 != 1 {
        eprintln!("usage: {} file.wav", args[0]);
        process::exit(-1);
    }

    let reader = WavReader::open(&args[1]).unwrap();
    let file_spec = reader.spec();

    if file_spec.sample_rate != JITTERS_SAMPLE_RATE {
        eprintln!("Input clip must have sampling rate of {:#?}", JITTERS_SAMPLE_RATE);
        process::exit(-1);
    }

    let mut rtp_stream = RtpOutStream::new(file_spec.channels);

    let samples = reader.into_samples().filter_map(Result::ok).map(i16::to_sample::<f64>);
    let signal = signal::from_interleaved_samples_iter::<_, [f64; 1]>(samples);
    let chunkable_signal = signal.until_exhausted().collect::<Vec<_>>();
    let mut buf = vec![0u8; JITTERS_MAX_PACKET_SIZE];
    let mut time_in_ms = 0u32;

    for (i, frames) in chunkable_signal.chunks(JITTERS_MAX_PACKET_SIZE/2).enumerate() {
        for (j, frame) in frames.iter().enumerate() {
            let single_sample = frame[0].to_sample::<i16>();
            NetworkEndian::write_i16(&mut buf[j*2..], single_sample);
        }
        let next_packet = rtp_stream.next_packet(&buf, time_in_ms);
        println!("Sent samples at timestamp {:#?}ms with RTP over UDP to 127.0.0.1:1337", time_in_ms);
        udp_sock.send_to(&next_packet, "127.0.0.1:1337").unwrap();
        process::exit(0);
        time_in_ms = ((i as f64) * ((1000.0 / (JITTERS_SAMPLE_RATE as f64)) * (frames.len() as f64))) as u32;
    }
}
