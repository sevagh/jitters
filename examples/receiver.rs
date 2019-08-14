use jitters::MAX_PACKET_SIZE;
use std::net::UdpSocket;

fn main() {
    // mono channel
    let socket = UdpSocket::bind("127.0.0.1:1234").unwrap();

    let mut buf = [0; MAX_PACKET_SIZE];

    loop {
        let (amt, src) = socket.recv_from(&mut buf).unwrap();
        println!("amt: {:#?}", amt);
        println!("src: {:#?}", src);
        //println!("buf: {:#?}", buf);
    }
}
