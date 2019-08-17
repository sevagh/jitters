use crossbeam::{
    queue::{ArrayQueue, PopError, PushError},
    utils::Backoff,
};
use jitters::rtp::{RtpHeader, RtpInStream, JITTERS_MAX_PACKET_SIZE, JITTERS_SAMPLE_RATE};
use std::{env, mem::size_of, net::UdpSocket, process, sync::Arc, thread};

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

    let get_packet_queue = packet_queue.clone(); // "get" ref to the packet_queue
    let getter_thread = thread::spawn(move || {
        let mut rtp_stream: Option<RtpInStream> = None;
        'outer: loop {
            let backoff = Backoff::new();
            'inner: loop {
                match get_packet_queue.pop() {
                    Ok(packet) => {
                        if let Some(ref mut rtp_stream_) = rtp_stream {
                            rtp_stream_.next_packet(&packet);
                        } else {
                            rtp_stream = Some(RtpInStream::new(&packet));
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

    putter_thread.join().expect("udp receiver thread panicked");
    getter_thread.join().expect("rtp in-stream thread panicked");
}
