use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::{io::Result, net::UdpSocket, time::Duration};

const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 1);
const IFACE: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
const ADDR: SocketAddrV4 = SocketAddrV4::new(IFACE, 5123);

async fn listen() -> Result<()> {
    let socket = UdpSocket::bind(ADDR).await?;
    socket.join_multicast_v4(MULTICAST_ADDR, IFACE)?;

    loop {
        let mut buf = [0u8; 1024];
        let (len, source) = socket.recv_from(&mut buf).await?;
        println!("REQUEST: {source} -> {len} bytes")
    }

    Ok(())
}

async fn cast() -> Result<()> {
    let socket = UdpSocket::bind(SocketAddrV4::new(IFACE, 0)).await?;
    socket
        .connect(SocketAddrV4::new(MULTICAST_ADDR, 5123))
        .await?;
    socket.set_multicast_loop_v4(false)?;

    let data = "hi";
    loop {
        println!("My IP: {}", socket.local_addr().unwrap().ip());
        socket.send(data.as_bytes()).await?;
        std::thread::sleep(Duration::from_secs(10));
    }

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Message {}

#[tokio::main]
async fn main() {
    tokio::spawn(async {
        cast().await.unwrap();
    });
    listen().await.unwrap();
}
