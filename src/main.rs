use std::{
    io::Result,
    net::{self, IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket},
    time::Duration,
};

const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 1);
const IFACE: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
const ADDR: SocketAddrV4 = SocketAddrV4::new(IFACE, 5123);

fn listen() -> Result<()> {
    let socket = UdpSocket::bind(ADDR)?;
    socket.join_multicast_v4(&MULTICAST_ADDR, &IFACE)?;

    loop {
        let mut buf = [0u8; 1024];
        let (len, source) = socket.recv_from(&mut buf)?;
        println!("REQUEST: {source} -> {len} bytes")
    }

    Ok(())
}

fn cast() -> Result<()> {
    let socket = UdpSocket::bind(ADDR)?;
    socket.connect(SocketAddrV4::new(MULTICAST_ADDR, 5123))?;

    let data = "hi";

    loop {
        socket.send(data.as_bytes())?;
        std::thread::sleep(Duration::from_secs(10));
    }

    Ok(())
}

fn main() {
    std::thread::spawn(|| {
        cast();
    });
    listen().unwrap();
}
