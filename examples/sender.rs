use hound::WavReader;
use jitters::{RtpOutStream, JITTERS_MAX_PACKET_SIZE, JITTERS_SAMPLE_RATE};
use sample::{interpolate::Linear, signal, Signal};
use std::{env, net::UdpSocket, process};

fn main() {
    let udp_sock = UdpSocket::bind("127.0.0.1:13337").unwrap();

    let args: Vec<String> = env::args().collect();
    if args.len() - 1 != 1 {
        eprintln!("usage: {} file.wav", args[0]);
        process::exit(-1);
    }

    let mut reader = WavReader::open(args[1]).unwrap();
    let file_spec = reader.spec();

    let mut rtp_stream = RtpOutStream::new(file_spec.channels);

    let num_samples = reader.len() as usize;
    let signal = reader
        .samples::<i16>()
        .map(|x| [x.unwrap()])
        .collect::<Vec<_>>();

    let mut source = signal::from_iter(signal.iter().cloned());
    let interp = Linear::from_source(&mut source);
    let fixed_frames = source
        .from_hz_to_hz(interp, file_spec.sample_rate as f64, JITTERS_SAMPLE_RATE)
        .take(8)
        //.iter()
        .map(|x| x[0])
        .collect::<Vec<_>>();

    for (i, frame) in fixed_frames.chunks(JITTERS_MAX_PACKET_SIZE).enumerate() {
        let time_in_s = ((i as f64) * ((1.0 / JITTERS_SAMPLE_RATE) * (frame.len() as f64))) as u32;
        let next_packet = rtp_stream.next_packet(frame, time_in_s);
        udp_sock.send_to(&next_packet, "127.0.0.1:1337");
    }
}
