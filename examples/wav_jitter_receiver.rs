#![feature(generators, generator_trait)]

use byteorder::{ByteOrder, NetworkEndian};
use cpal::{
    self,
    traits::{EventLoopTrait, HostTrait},
};
use crossbeam::{
    queue::{ArrayQueue, PopError, PushError},
    utils::Backoff,
};
use jitters::{
    rtp::{RtpHeader, JITTERS_MAX_PACKET_SIZE, JITTERS_SAMPLE_RATE},
    rtp_jitter::RtpJitterInStream,
    util::samples_to_ms,
};
use std::{
    env, mem,
    mem::size_of,
    net::UdpSocket,
    ops::{Generator, GeneratorState},
    pin::Pin,
    process,
    sync::{Arc, RwLock},
    thread,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() - 1 != 1 {
        eprintln!("usage: {} listenhostport", args[0]);
        process::exit(-1);
    }
    let listenhostport = String::from(&args[1]);
    let packet_queue = Arc::new(ArrayQueue::<Vec<u8>>::new(1000));

    let put_packet_queue = packet_queue.clone(); // "put" ref to the packet_queue
    let putter_thread = thread::spawn(move || {
        let mut buf = [0u8; JITTERS_MAX_PACKET_SIZE + size_of::<RtpHeader>()];
        let udp_sock = UdpSocket::bind(listenhostport).unwrap();
        loop {
            let (amt, src) = udp_sock.recv_from(&mut buf).unwrap();
            println!("Received {:#?} bytes from {}:{}", amt, src.ip(), src.port());
            match put_packet_queue.push(buf.to_vec()) {
                Ok(()) => {}
                Err(PushError(_)) => panic!("couldn't push next packet onto the queue"),
            }
        }
    });

    let rtp_stream: Arc<RwLock<Option<RtpJitterInStream>>> = Arc::new(RwLock::new(None));

    let get_packet_queue = packet_queue.clone(); // "get" ref to the packet_queue
    let put_rtp_stream = rtp_stream.clone(); // "put" ref to the RtpJitterInStream
    let getter_thread = thread::spawn(move || {
        'outer: loop {
            let backoff = Backoff::new();
            'inner: loop {
                let mut guard = put_rtp_stream.write().unwrap();
                match get_packet_queue.pop() {
                    Ok(packet) => {
                        if let Some(ref mut rtp_stream_) = *guard {
                            rtp_stream_.next_packet(&packet);
                        } else {
                            mem::replace(&mut *guard, Some(RtpJitterInStream::new(&packet)));
                        }
                        continue 'outer;
                    }
                    Err(PopError) => {
                        //println!("Waiting for more packets...");
                        backoff.snooze();
                        continue 'inner;
                    }
                }
            }
        }
    });

    let play_rtp_stream = rtp_stream.clone(); // "play" ref to the RtpJitterInStream
    let player_thread = thread::spawn(move || {
        loop {
            let backoff = Backoff::new();
            let guard = play_rtp_stream.read().unwrap();
            if let Some(ref rtp_stream_) = *guard {
                if rtp_stream_.ended() {
                    // play
                    println!("Stream ended - playing audio...");

                    println!("Corrected {:#?} out-of-order packets", rtp_stream_.jitter());

                    let host = cpal::default_host();
                    let event_loop = host.event_loop();
                    let device = host
                        .default_output_device()
                        .expect("no output device available");

                    let format = cpal::Format {
                        channels: rtp_stream_.channels as cpal::ChannelCount,
                        sample_rate: cpal::SampleRate(JITTERS_SAMPLE_RATE),
                        data_type: cpal::SampleFormat::I16,
                    };

                    let stream_id = event_loop.build_output_stream(&device, &format).unwrap();

                    event_loop
                        .play_stream(stream_id.clone())
                        .expect("couldn't play_stream on event_loop");

                    let mut next_value_generator = || {
                        for audio_info in rtp_stream_.audio_slices.iter() {
                            let (audio_slice, seq, timestamp) = audio_info;
                            println!(
                                "Yielding audio slice for sequence {:#?}, timestamp {:#?}ms",
                                seq,
                                samples_to_ms(*timestamp as usize, rtp_stream_.channels)
                            );
                            for audio_slice_slice in
                                audio_slice.chunks(2 * rtp_stream_.channels as usize)
                            {
                                let mut ret = vec![0i16; rtp_stream_.channels as usize];
                                match rtp_stream_.channels {
                                    1 => {
                                        ret[0] = NetworkEndian::read_i16(&audio_slice_slice);
                                    }
                                    2 => {
                                        ret[0] = NetworkEndian::read_i16(&audio_slice_slice);
                                        ret[1] = NetworkEndian::read_i16(&audio_slice_slice[2..]);
                                    }
                                    _ => panic!("come on bruh"),
                                }
                                yield ret;
                            }
                        }
                        return;
                    };

                    event_loop.run(move |id, result| {
                        let data = match result {
                            Ok(data) => data,
                            Err(err) => {
                                eprintln!("an error occurred on stream {:?}: {}", id, err);
                                return;
                            }
                        };

                        match data {
                            cpal::StreamData::Output {
                                buffer: cpal::UnknownTypeOutputBuffer::I16(mut buffer),
                            } => {
                                for sample in buffer.chunks_mut(format.channels as usize) {
                                    match Pin::new(&mut next_value_generator).resume() {
                                        GeneratorState::Yielded(mut x) => {
                                            for out in sample.iter_mut() {
                                                for x_ in x.iter_mut() {
                                                    *out = *x_;
                                                }
                                            }
                                        }
                                        GeneratorState::Complete(()) => {
                                            println!("audio done, exiting program");
                                            process::exit(0);
                                        }
                                    }
                                }
                            }
                            _ => (),
                        }
                    });
                }
            }
            backoff.snooze();
        }
    });

    putter_thread.join().expect("udp receiver thread panicked");
    getter_thread.join().expect("rtp in-stream thread panicked");
    player_thread.join().expect("rtp player thread panicked");
}
